use openapiv3::{
    Components, ObjectType, OpenAPI, Operation, Parameter, ParameterSchemaOrContent, PathItem,
    ReferenceOr, RequestBody, Response, Responses, Schema, SchemaKind, StatusCode, StringType,
    Type,
};

use std::ops::Index;

use crate::change::{Change, Location, Severity};

#[derive(Debug)]
pub struct Diff(Vec<Change>);

impl Diff {
    pub fn new(changes: Vec<Change>) -> Self {
        Self(changes)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn has_breaking(&self) -> bool {
        self.0.iter().any(|c| c.severity == Severity::Breaking)
    }

    pub fn breaking(&self) -> Vec<&Change> {
        self.0
            .iter()
            .filter(|c| c.severity == Severity::Breaking)
            .collect()
    }

    pub fn non_breaking(&self) -> Vec<&Change> {
        self.0
            .iter()
            .filter(|c| c.severity == Severity::NonBreaking)
            .collect()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Index<usize> for Diff {
    type Output = Change;

    fn index(&self, index: usize) -> &Change {
        &self.0[index]
    }
}

/// Compare two OpenAPI specs and return a list of changes.
pub fn diff_specs(old: &OpenAPI, new: &OpenAPI) -> Diff {
    Diff::new(diff_paths(old, new))
}

// ---------------------------------------------------------------------------
// Layer 1: Paths
// ---------------------------------------------------------------------------

fn diff_paths(old: &OpenAPI, new: &OpenAPI) -> Vec<Change> {
    let removed = old.paths.iter().filter_map(|(path, _)| {
        if new.paths.paths.contains_key(path) {
            None
        } else {
            Some(Change {
                severity: Severity::Breaking,
                location: Location::Path(path.clone()),
                message: "endpoint removed".into(),
            })
        }
    });

    let added = new.paths.paths.keys().filter_map(|path| {
        if old.paths.paths.contains_key(path) {
            None
        } else {
            Some(Change {
                severity: Severity::NonBreaking,
                location: Location::Path(path.clone()),
                message: "endpoint added".into(),
            })
        }
    });

    let shared = old.paths.iter().flat_map(|(path, old_ref)| {
        new.paths
            .paths
            .get(path)
            .into_iter()
            .flat_map(
                move |new_ref| match (old_ref.as_item(), new_ref.as_item()) {
                    (Some(old_item), Some(new_item)) => {
                        diff_path_item(path, old_item, new_item, old, new)
                    }
                    _ => vec![],
                },
            )
    });

    removed.chain(added).chain(shared).collect()
}

// ---------------------------------------------------------------------------
// Layer 2: PathItem (operations per HTTP method)
// ---------------------------------------------------------------------------

fn operations(item: &PathItem) -> [(&str, &Option<Operation>); 8] {
    [
        ("GET", &item.get),
        ("PUT", &item.put),
        ("POST", &item.post),
        ("DELETE", &item.delete),
        ("OPTIONS", &item.options),
        ("HEAD", &item.head),
        ("PATCH", &item.patch),
        ("TRACE", &item.trace),
    ]
}

fn diff_path_item(
    path: &str,
    old: &PathItem,
    new: &PathItem,
    old_spec: &OpenAPI,
    new_spec: &OpenAPI,
) -> Vec<Change> {
    operations(old)
        .into_iter()
        .zip(operations(new))
        .flat_map(|((method, old_op), (_, new_op))| {
            let location = Location::Operation {
                path: path.to_string(),
                method: method.to_string(),
            };
            match (old_op, new_op) {
                (Some(_), None) => vec![Change {
                    severity: Severity::Breaking,
                    location,
                    message: "operation removed".into(),
                }],
                (None, Some(_)) => vec![Change {
                    severity: Severity::NonBreaking,
                    location,
                    message: "operation added".into(),
                }],
                (Some(old_op), Some(new_op)) => {
                    diff_operation(&location, old_op, new_op, old_spec, new_spec)
                }
                (None, None) => vec![],
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Layer 3: Operation
// ---------------------------------------------------------------------------

fn diff_operation(
    loc: &Location,
    old: &Operation,
    new: &Operation,
    old_spec: &OpenAPI,
    new_spec: &OpenAPI,
) -> Vec<Change> {
    let params = diff_parameters(
        loc,
        &old.parameters,
        &new.parameters,
        &old_spec.components,
        &new_spec.components,
    );
    let body = diff_request_body(
        loc,
        &old.request_body,
        &new.request_body,
        old_spec,
        new_spec,
    );
    let responses = diff_responses(loc, &old.responses, &new.responses, old_spec, new_spec);

    let deprecated = if !old.deprecated && new.deprecated {
        Some(Change {
            severity: Severity::NonBreaking,
            location: loc.clone(),
            message: "operation deprecated".into(),
        })
    } else {
        None
    };

    params
        .into_iter()
        .chain(body)
        .chain(responses)
        .chain(deprecated)
        .collect()
}

// ---------------------------------------------------------------------------
// Layer 4: Parameters
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Hash)]
struct ParamKey {
    name: String,
    location: String,
}

fn param_key(p: &Parameter) -> ParamKey {
    let data = p.parameter_data_ref();
    let location = match p {
        Parameter::Query { .. } => "query",
        Parameter::Header { .. } => "header",
        Parameter::Path { .. } => "path",
        Parameter::Cookie { .. } => "cookie",
    };
    ParamKey {
        name: data.name.clone(),
        location: location.to_string(),
    }
}

fn resolve_param<'a>(
    r: &'a ReferenceOr<Parameter>,
    components: &'a Option<Components>,
) -> Option<&'a Parameter> {
    match r {
        ReferenceOr::Item(p) => Some(p),
        ReferenceOr::Reference { reference } => reference
            .strip_prefix("#/components/parameters/")
            .and_then(|name| {
                components
                    .as_ref()
                    .and_then(|c| c.parameters.get(name))
                    .and_then(|r| r.as_item())
            }),
    }
}

fn diff_parameters(
    loc: &Location,
    old_params: &[ReferenceOr<Parameter>],
    new_params: &[ReferenceOr<Parameter>],
    old_components: &Option<Components>,
    new_components: &Option<Components>,
) -> Vec<Change> {
    let old_map: std::collections::HashMap<ParamKey, &Parameter> = old_params
        .iter()
        .filter_map(|r| resolve_param(r, old_components))
        .map(|p| (param_key(p), p))
        .collect();

    let new_map: std::collections::HashMap<ParamKey, &Parameter> = new_params
        .iter()
        .filter_map(|r| resolve_param(r, new_components))
        .map(|p| (param_key(p), p))
        .collect();

    let existing = old_map.iter().flat_map(|(key, old_p)| {
        let old_data = old_p.parameter_data_ref();
        match new_map.get(key) {
            None => {
                vec![Change {
                    severity: if old_data.required {
                        Severity::Breaking
                    } else {
                        Severity::NonBreaking
                    },
                    location: loc.clone(),
                    message: format!("{} parameter '{}' removed", key.location, key.name),
                }]
            }
            Some(new_p) => {
                let new_data = new_p.parameter_data_ref();
                let mut changes = Vec::new();
                if !old_data.required && new_data.required {
                    changes.push(Change {
                        severity: Severity::Breaking,
                        location: loc.clone(),
                        message: format!("parameter '{}' became required", key.name),
                    });
                }
                if old_data.required && !new_data.required {
                    changes.push(Change {
                        severity: Severity::NonBreaking,
                        location: loc.clone(),
                        message: format!("parameter '{}' became optional", key.name),
                    });
                }
                changes.extend(diff_parameter_type(
                    loc,
                    &key.name,
                    &old_data.format,
                    &new_data.format,
                ));
                changes
            }
        }
    });

    let added = new_map.iter().filter_map(|(key, new_p)| {
        if old_map.contains_key(key) {
            None
        } else {
            let new_data = new_p.parameter_data_ref();
            let sev = if new_data.required {
                Severity::Breaking
            } else {
                Severity::NonBreaking
            };
            Some(Change {
                severity: sev,
                location: loc.clone(),
                message: format!("{} parameter '{}' added", key.location, key.name),
            })
        }
    });

    existing.chain(added).collect()
}

fn diff_parameter_type(
    loc: &Location,
    name: &str,
    old_format: &ParameterSchemaOrContent,
    new_format: &ParameterSchemaOrContent,
) -> Vec<Change> {
    let old_schema = match old_format {
        ParameterSchemaOrContent::Schema(r) => r.as_item(),
        _ => None,
    };
    let new_schema = match new_format {
        ParameterSchemaOrContent::Schema(r) => r.as_item(),
        _ => None,
    };
    match (old_schema, new_schema) {
        (Some(old_s), Some(new_s))
            if type_name(&old_s.schema_kind) != type_name(&new_s.schema_kind) =>
        {
            vec![Change {
                severity: Severity::Breaking,
                location: loc.clone(),
                message: format!(
                    "parameter '{}' type changed from {} to {}",
                    name,
                    type_name(&old_s.schema_kind),
                    type_name(&new_s.schema_kind),
                ),
            }]
        }
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// Layer 5: Request Body
// ---------------------------------------------------------------------------

fn resolve_request_body<'a>(
    r: &'a ReferenceOr<RequestBody>,
    spec: &'a OpenAPI,
) -> Option<&'a RequestBody> {
    match r {
        ReferenceOr::Item(rb) => Some(rb),
        ReferenceOr::Reference { reference } => {
            let prefix = "#/components/requestBodies/";
            reference.strip_prefix(prefix).and_then(|name| {
                spec.components
                    .as_ref()
                    .and_then(|c| c.request_bodies.get(name))
                    .and_then(|r| r.as_item())
            })
        }
    }
}

fn diff_request_body(
    loc: &Location,
    old: &Option<ReferenceOr<RequestBody>>,
    new: &Option<ReferenceOr<RequestBody>>,
    old_spec: &OpenAPI,
    new_spec: &OpenAPI,
) -> Vec<Change> {
    let old_rb = old.as_ref().and_then(|r| resolve_request_body(r, old_spec));
    let new_rb = new.as_ref().and_then(|r| resolve_request_body(r, new_spec));

    match (old_rb, new_rb) {
        (None, Some(rb)) => {
            let sev = if rb.required {
                Severity::Breaking
            } else {
                Severity::NonBreaking
            };
            vec![Change {
                severity: sev,
                location: loc.clone(),
                message: "request body added".into(),
            }]
        }
        (Some(_), None) => {
            vec![Change {
                severity: Severity::Breaking,
                location: loc.clone(),
                message: "request body removed".into(),
            }]
        }
        (Some(old_rb), Some(new_rb)) => {
            let required = if !old_rb.required && new_rb.required {
                Some(Change {
                    severity: Severity::Breaking,
                    location: loc.clone(),
                    message: "request body became required".into(),
                })
            } else {
                None
            };
            let content = diff_content(
                loc,
                "request body",
                &old_rb.content,
                &new_rb.content,
                Direction::Request,
                old_spec,
                new_spec,
            );
            required.into_iter().chain(content).collect()
        }
        (None, None) => vec![],
    }
}

// ---------------------------------------------------------------------------
// Layer 6: Responses
// ---------------------------------------------------------------------------

fn resolve_response<'a>(r: &'a ReferenceOr<Response>, spec: &'a OpenAPI) -> Option<&'a Response> {
    match r {
        ReferenceOr::Item(resp) => Some(resp),
        ReferenceOr::Reference { reference } => {
            let prefix = "#/components/responses/";
            reference.strip_prefix(prefix).and_then(|name| {
                spec.components
                    .as_ref()
                    .and_then(|c| c.responses.get(name))
                    .and_then(|r| r.as_item())
            })
        }
    }
}

fn status_code_str(sc: &StatusCode) -> String {
    match sc {
        StatusCode::Code(c) => c.to_string(),
        StatusCode::Range(r) => format!("{}XX", r),
    }
}

fn diff_responses(
    loc: &Location,
    old: &Responses,
    new: &Responses,
    old_spec: &OpenAPI,
    new_spec: &OpenAPI,
) -> Vec<Change> {
    let existing = old
        .responses
        .iter()
        .flat_map(|(code, old_ref)| match new.responses.get(code) {
            None => {
                vec![Change {
                    severity: Severity::Breaking,
                    location: loc.clone(),
                    message: format!("response '{}' removed", status_code_str(code)),
                }]
            }
            Some(new_ref) => {
                match (
                    resolve_response(old_ref, old_spec),
                    resolve_response(new_ref, new_spec),
                ) {
                    (Some(old_resp), Some(new_resp)) => {
                        let label = format!("response '{}'", status_code_str(code));
                        diff_content(
                            loc,
                            &label,
                            &old_resp.content,
                            &new_resp.content,
                            Direction::Response,
                            old_spec,
                            new_spec,
                        )
                    }
                    _ => vec![],
                }
            }
        });

    let added = new.responses.keys().filter_map(|code| {
        if old.responses.contains_key(code) {
            None
        } else {
            Some(Change {
                severity: Severity::NonBreaking,
                location: loc.clone(),
                message: format!("response '{}' added", status_code_str(code)),
            })
        }
    });

    existing.chain(added).collect()
}

// ---------------------------------------------------------------------------
// Content (shared between request body and responses)
// ---------------------------------------------------------------------------

use indexmap::IndexMap;
use openapiv3::MediaType;

#[derive(Debug, Clone, Copy)]
enum Direction {
    Request,
    Response,
}

fn diff_content(
    loc: &Location,
    label: &str,
    old_content: &IndexMap<String, MediaType>,
    new_content: &IndexMap<String, MediaType>,
    direction: Direction,
    old_spec: &OpenAPI,
    new_spec: &OpenAPI,
) -> Vec<Change> {
    let existing = old_content
        .iter()
        .flat_map(|(media, old_mt)| match new_content.get(media) {
            None => {
                vec![Change {
                    severity: Severity::Breaking,
                    location: loc.clone(),
                    message: format!("{label}: media type '{media}' removed"),
                }]
            }
            Some(new_mt) => {
                let old_schema = old_mt
                    .schema
                    .as_ref()
                    .and_then(|r| resolve_schema(r, &old_spec.components));
                let new_schema = new_mt
                    .schema
                    .as_ref()
                    .and_then(|r| resolve_schema(r, &new_spec.components));

                match (old_schema, new_schema) {
                    (Some(old_s), Some(new_s)) => {
                        let ctx = format!("{label} {media}");
                        diff_schema(
                            loc,
                            &ctx,
                            old_s,
                            new_s,
                            direction,
                            &old_spec.components,
                            &new_spec.components,
                            0,
                        )
                    }
                    _ => vec![],
                }
            }
        });

    let added = new_content.keys().filter_map(|media| {
        if old_content.contains_key(media) {
            None
        } else {
            Some(Change {
                severity: Severity::NonBreaking,
                location: loc.clone(),
                message: format!("{label}: media type '{media}' added"),
            })
        }
    });

    existing.chain(added).collect()
}

// ---------------------------------------------------------------------------
// Layer 7: Schema comparison
// ---------------------------------------------------------------------------

const MAX_DEPTH: usize = 10;

fn resolve_schema<'a>(
    r: &'a ReferenceOr<Schema>,
    components: &'a Option<Components>,
) -> Option<&'a Schema> {
    match r {
        ReferenceOr::Item(s) => Some(s),
        ReferenceOr::Reference { reference } => {
            let prefix = "#/components/schemas/";
            reference.strip_prefix(prefix).and_then(|name| {
                components
                    .as_ref()
                    .and_then(|c| c.schemas.get(name))
                    .and_then(|r| r.as_item())
            })
        }
    }
}

fn resolve_box_schema<'a>(
    r: &'a ReferenceOr<Box<Schema>>,
    components: &'a Option<Components>,
) -> Option<&'a Schema> {
    match r {
        ReferenceOr::Item(s) => Some(s.as_ref()),
        ReferenceOr::Reference { reference } => {
            let prefix = "#/components/schemas/";
            reference.strip_prefix(prefix).and_then(|name| {
                components
                    .as_ref()
                    .and_then(|c| c.schemas.get(name))
                    .and_then(|r| r.as_item())
            })
        }
    }
}

fn type_name(kind: &SchemaKind) -> String {
    match kind {
        SchemaKind::Type(t) => match t {
            Type::String(_) => "string".into(),
            Type::Number(_) => "number".into(),
            Type::Integer(_) => "integer".into(),
            Type::Object(_) => "object".into(),
            Type::Array(_) => "array".into(),
            Type::Boolean(_) => "boolean".into(),
        },
        SchemaKind::OneOf { .. } => "oneOf".into(),
        SchemaKind::AllOf { .. } => "allOf".into(),
        SchemaKind::AnyOf { .. } => "anyOf".into(),
        SchemaKind::Not { .. } => "not".into(),
        SchemaKind::Any(_) => "any".into(),
    }
}

fn diff_schema(
    loc: &Location,
    context: &str,
    old: &Schema,
    new: &Schema,
    direction: Direction,
    old_components: &Option<Components>,
    new_components: &Option<Components>,
    depth: usize,
) -> Vec<Change> {
    if depth >= MAX_DEPTH {
        return vec![];
    }

    let old_type = type_name(&old.schema_kind);
    let new_type = type_name(&new.schema_kind);

    if old_type != new_type {
        return vec![Change {
            severity: Severity::Breaking,
            location: loc.clone(),
            message: format!("{context}: type changed from {old_type} to {new_type}"),
        }];
    }

    match (&old.schema_kind, &new.schema_kind) {
        (SchemaKind::Type(Type::Object(old_obj)), SchemaKind::Type(Type::Object(new_obj))) => {
            diff_object(
                loc,
                context,
                old_obj,
                new_obj,
                direction,
                old_components,
                new_components,
                depth,
            )
        }
        (SchemaKind::Type(Type::Array(old_arr)), SchemaKind::Type(Type::Array(new_arr))) => {
            let old_items = old_arr
                .items
                .as_ref()
                .and_then(|r| resolve_box_schema(r, old_components));
            let new_items = new_arr
                .items
                .as_ref()
                .and_then(|r| resolve_box_schema(r, new_components));
            match (old_items, new_items) {
                (Some(old_s), Some(new_s)) => diff_schema(
                    loc,
                    &format!("{context}[]"),
                    old_s,
                    new_s,
                    direction,
                    old_components,
                    new_components,
                    depth + 1,
                ),
                _ => vec![],
            }
        }
        (SchemaKind::Type(Type::String(old_s)), SchemaKind::Type(Type::String(new_s))) => {
            diff_string_enum(loc, context, old_s, new_s, direction)
        }
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// Object property comparison
// ---------------------------------------------------------------------------

fn diff_object(
    loc: &Location,
    context: &str,
    old: &ObjectType,
    new: &ObjectType,
    direction: Direction,
    old_components: &Option<Components>,
    new_components: &Option<Components>,
    depth: usize,
) -> Vec<Change> {
    let removed = old.properties.keys().filter_map(|prop_name| {
        if new.properties.contains_key(prop_name) {
            None
        } else {
            let sev = match direction {
                Direction::Response => Severity::Breaking,
                Direction::Request => Severity::NonBreaking,
            };
            Some(Change {
                severity: sev,
                location: loc.clone(),
                message: format!("{context}: property '{prop_name}' removed"),
            })
        }
    });

    let added = new.properties.keys().filter_map(|prop_name| {
        if old.properties.contains_key(prop_name) {
            None
        } else {
            let is_required = new.required.contains(prop_name);
            let sev = match direction {
                Direction::Request if is_required => Severity::Breaking,
                _ => Severity::NonBreaking,
            };
            Some(Change {
                severity: sev,
                location: loc.clone(),
                message: format!("{context}: property '{prop_name}' added"),
            })
        }
    });

    let became_required = new.required.iter().filter_map(|prop_name| {
        if !old.required.contains(prop_name) && old.properties.contains_key(prop_name) {
            let sev = match direction {
                Direction::Request => Severity::Breaking,
                Direction::Response => Severity::NonBreaking,
            };
            Some(Change {
                severity: sev,
                location: loc.clone(),
                message: format!("{context}: property '{prop_name}' became required"),
            })
        } else {
            None
        }
    });

    let became_optional = old.required.iter().filter_map(|prop_name| {
        if !new.required.contains(prop_name) && new.properties.contains_key(prop_name) {
            let sev = match direction {
                Direction::Request => Severity::NonBreaking,
                Direction::Response => Severity::Breaking,
            };
            Some(Change {
                severity: sev,
                location: loc.clone(),
                message: format!("{context}: property '{prop_name}' became optional"),
            })
        } else {
            None
        }
    });

    let recursed = old.properties.iter().flat_map(|(prop_name, old_ref)| {
        new.properties
            .get(prop_name)
            .into_iter()
            .flat_map(move |new_ref| {
                let old_schema = resolve_box_schema(old_ref, old_components);
                let new_schema = resolve_box_schema(new_ref, new_components);
                match (old_schema, new_schema) {
                    (Some(old_s), Some(new_s)) => diff_schema(
                        loc,
                        &format!("{context}.{prop_name}"),
                        old_s,
                        new_s,
                        direction,
                        old_components,
                        new_components,
                        depth + 1,
                    ),
                    _ => vec![],
                }
            })
    });

    removed
        .chain(added)
        .chain(became_required)
        .chain(became_optional)
        .chain(recursed)
        .collect()
}

// ---------------------------------------------------------------------------
// String enum comparison
// ---------------------------------------------------------------------------

fn diff_string_enum(
    loc: &Location,
    context: &str,
    old: &StringType,
    new: &StringType,
    direction: Direction,
) -> Vec<Change> {
    if old.enumeration.is_empty() && new.enumeration.is_empty() {
        return vec![];
    }

    let old_values: std::collections::HashSet<_> =
        old.enumeration.iter().filter_map(|v| v.as_ref()).collect();
    let new_values: std::collections::HashSet<_> =
        new.enumeration.iter().filter_map(|v| v.as_ref()).collect();

    let removed = old_values.iter().filter_map(|val| {
        if new_values.contains(val) {
            None
        } else {
            Some(Change {
                severity: Severity::Breaking,
                location: loc.clone(),
                message: format!("{context}: enum value '{val}' removed"),
            })
        }
    });

    let added = new_values.iter().filter_map(|val| {
        if old_values.contains(val) {
            None
        } else {
            let sev = match direction {
                Direction::Response => Severity::Breaking,
                Direction::Request => Severity::NonBreaking,
            };
            Some(Change {
                severity: sev,
                location: loc.clone(),
                message: format!("{context}: enum value '{val}' added"),
            })
        }
    });

    removed.chain(added).collect()
}

#[cfg(test)]
mod tests;
