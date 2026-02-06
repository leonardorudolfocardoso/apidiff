# apidiff

A CLI tool for comparing OpenAPI 3.0.x specifications and detecting breaking changes.

## Install

```bash
cargo install --path .
```

## Usage

```bash
apidiff <old-spec> <new-spec>
```

Supports both YAML and JSON specs.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | No breaking changes |
| 1 | Breaking changes found |
| 2 | Error (bad file, invalid spec) |

### Example

```
$ apidiff old.yaml new.yaml

Breaking changes (3):
  DELETE /pets/{petId} - operation removed
  GET /pets/{petId} - parameter 'petId' type changed from string to integer
  POST /pets - request body application/json: property 'species' added

Non-breaking changes (2):
  /pets/{petId}/toys - endpoint added
  GET /pets/{petId} - operation deprecated
```

## What it detects

apidiff walks the spec top-down through 7 layers:

1. **Paths** - endpoints added/removed
2. **Operations** - HTTP methods added/removed/deprecated
3. **Parameters** - added/removed, required/optional changes, type changes
4. **Request bodies** - added/removed, required changes
5. **Responses** - status codes added/removed
6. **Content** - media types added/removed
7. **Schemas** - type changes, properties, enums with `$ref` resolution

Breaking change rules are **direction-aware**: removing a required request property is non-breaking (clients just stop sending it), but removing a response property is breaking (clients may depend on it).

## Limitations

- **OpenAPI 3.0.x only** - no support for OpenAPI 3.1 or Swagger 2.0
- **No schema composition diffing** - `allOf`, `oneOf`, and `anyOf` schemas are not compared
- **No `additionalProperties` tracking** - changes to additional properties are not detected
- **No response header/cookie changes** - only response body content is compared
- **No security scheme diffing** - changes to authentication and authorization are not detected
- **No `$ref` cycle detection** - relies on a depth limit (10) to prevent infinite recursion

## Build

```bash
cargo build --release
cargo test
```

## Acknowledgments

Built with [Claude Code](https://claude.ai/code) as a coding partner for implementation, architecture decisions, and code review.
