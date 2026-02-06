use super::*;
use crate::change::{Location, Severity};

fn parse_spec(yaml: &str) -> OpenAPI {
    serde_yml::from_str(yaml).expect("test spec should parse")
}

fn minimal_spec(paths_yaml: &str) -> String {
    format!(
        r#"
openapi: "3.0.3"
info:
  title: Test
  version: "1.0.0"
{paths_yaml}
"#
    )
}

#[test]
fn no_changes_for_identical_specs() {
    let yaml = minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
"#,
    );
    let spec = parse_spec(&yaml);
    let changes = diff_specs(&spec, &spec);
    assert!(changes.is_empty());
}

#[test]
fn endpoint_removed_is_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
"#,
    ));
    let new = parse_spec(&minimal_spec("paths: {}"));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::Breaking);
    assert!(changes[0].message.contains("endpoint removed"));
}

#[test]
fn endpoint_added_is_non_breaking() {
    let old = parse_spec(&minimal_spec("paths: {}"));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::NonBreaking);
    assert!(changes[0].message.contains("endpoint added"));
}

#[test]
fn operation_removed_is_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
    post:
      responses:
        "201":
          description: Created
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::Breaking);
    assert!(changes[0].message.contains("operation removed"));
    assert!(matches!(&changes[0].location, Location::Operation { method, .. } if method == "POST"));
}

#[test]
fn operation_added_is_non_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
    post:
      responses:
        "201":
          description: Created
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::NonBreaking);
    assert!(changes[0].message.contains("operation added"));
}

#[test]
fn required_parameter_added_is_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      parameters:
        - name: filter
          in: query
          required: true
          schema:
            type: string
      responses:
        "200":
          description: OK
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::Breaking);
    assert!(changes[0].message.contains("'filter'"));
}

#[test]
fn optional_parameter_added_is_non_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      parameters:
        - name: limit
          in: query
          required: false
          schema:
            type: integer
      responses:
        "200":
          description: OK
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::NonBreaking);
}

#[test]
fn parameter_became_required_is_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      parameters:
        - name: limit
          in: query
          required: false
          schema:
            type: integer
      responses:
        "200":
          description: OK
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      parameters:
        - name: limit
          in: query
          required: true
          schema:
            type: integer
      responses:
        "200":
          description: OK
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::Breaking);
    assert!(changes[0].message.contains("became required"));
}

#[test]
fn parameter_type_changed_is_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users/{id}:
    get:
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        "200":
          description: OK
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users/{id}:
    get:
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: integer
      responses:
        "200":
          description: OK
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::Breaking);
    assert!(changes[0].message.contains("type changed"));
}

#[test]
fn response_removed_is_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
        "404":
          description: Not Found
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::Breaking);
    assert!(changes[0].message.contains("response '404' removed"));
}

#[test]
fn response_property_removed_is_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
          content:
            application/json:
              schema:
                type: object
                properties:
                  id:
                    type: integer
                  name:
                    type: string
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
          content:
            application/json:
              schema:
                type: object
                properties:
                  id:
                    type: integer
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::Breaking);
    assert!(changes[0].message.contains("'name' removed"));
}

#[test]
fn request_body_required_property_added_is_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    post:
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required:
                - name
              properties:
                name:
                  type: string
      responses:
        "201":
          description: Created
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    post:
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required:
                - name
                - email
              properties:
                name:
                  type: string
                email:
                  type: string
      responses:
        "201":
          description: Created
"#,
    ));
    let diff = diff_specs(&old, &new);
    let breaking = diff.breaking();
    assert_eq!(breaking.len(), 1);
    assert!(breaking[0].message.contains("'email' added"));
}

#[test]
fn enum_value_removed_is_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
          content:
            application/json:
              schema:
                type: object
                properties:
                  status:
                    type: string
                    enum:
                      - active
                      - inactive
                      - pending
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
          content:
            application/json:
              schema:
                type: object
                properties:
                  status:
                    type: string
                    enum:
                      - active
                      - inactive
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::Breaking);
    assert!(changes[0].message.contains("'pending' removed"));
}

#[test]
fn operation_deprecated_is_non_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      deprecated: true
      responses:
        "200":
          description: OK
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::NonBreaking);
    assert!(changes[0].message.contains("deprecated"));
}

#[test]
fn schema_type_changed_is_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
          content:
            application/json:
              schema:
                type: object
                properties:
                  count:
                    type: integer
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
          content:
            application/json:
              schema:
                type: object
                properties:
                  count:
                    type: string
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::Breaking);
    assert!(changes[0].message.contains("type changed"));
    assert!(changes[0].message.contains("integer"));
    assert!(changes[0].message.contains("string"));
}

#[test]
fn ref_resolution_works() {
    let old = parse_spec(&minimal_spec(
        r##"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/User"
components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: integer
        name:
          type: string
"##,
    ));
    let new = parse_spec(&minimal_spec(
        r##"
paths:
  /users:
    get:
      responses:
        "200":
          description: OK
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/User"
components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: integer
"##,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::Breaking);
    assert!(changes[0].message.contains("'name' removed"));
}

#[test]
fn request_property_removed_is_non_breaking() {
    let old = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    post:
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                name:
                  type: string
                bio:
                  type: string
      responses:
        "201":
          description: Created
"#,
    ));
    let new = parse_spec(&minimal_spec(
        r#"
paths:
  /users:
    post:
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                name:
                  type: string
      responses:
        "201":
          description: Created
"#,
    ));
    let changes = diff_specs(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].severity, Severity::NonBreaking);
    assert!(changes[0].message.contains("'bio' removed"));
}
