use super::*;
use std::io::Write;
use tempfile::NamedTempFile;

const MINIMAL_YAML: &str = r#"
openapi: "3.0.3"
info:
  title: Test
  version: "1.0.0"
paths: {}
"#;

const MINIMAL_JSON: &str = r#"{
  "openapi": "3.0.3",
  "info": { "title": "Test", "version": "1.0.0" },
  "paths": {}
}"#;

fn write_temp_file(content: &str, suffix: &str) -> NamedTempFile {
    let mut file = tempfile::Builder::new().suffix(suffix).tempfile().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

#[test]
fn detect_format_json() {
    assert!(matches!(
        detect_format(Path::new("spec.json")),
        Some(Format::Json)
    ));
}

#[test]
fn detect_format_yaml() {
    assert!(matches!(
        detect_format(Path::new("spec.yaml")),
        Some(Format::Yaml)
    ));
    assert!(matches!(
        detect_format(Path::new("spec.yml")),
        Some(Format::Yaml)
    ));
}

#[test]
fn detect_format_unknown() {
    assert!(detect_format(Path::new("spec.txt")).is_none());
    assert!(detect_format(Path::new("spec")).is_none());
}

#[test]
fn parse_content_json() {
    let result = parse_content(MINIMAL_JSON, Format::Json);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().info.title, "Test");
}

#[test]
fn parse_content_yaml() {
    let result = parse_content(MINIMAL_YAML, Format::Yaml);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().info.title, "Test");
}

#[test]
fn parse_content_invalid_json() {
    let result = parse_content("not json", Format::Json);
    assert!(matches!(result, Err(ParseError::Json(_))));
}

#[test]
fn parse_content_invalid_yaml() {
    let result = parse_content("not: valid: yaml: :", Format::Yaml);
    assert!(matches!(result, Err(ParseError::Yaml(_))));
}

#[test]
fn parse_unknown_content_detects_json() {
    let result = parse_unknown_content(MINIMAL_JSON);
    assert!(result.is_ok());
}

#[test]
fn parse_unknown_content_detects_yaml() {
    let result = parse_unknown_content(MINIMAL_YAML);
    assert!(result.is_ok());
}

#[test]
fn load_file_yaml() {
    let file = write_temp_file(MINIMAL_YAML, ".yaml");
    let result = load_file(file.path());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().info.title, "Test");
}

#[test]
fn load_file_json() {
    let file = write_temp_file(MINIMAL_JSON, ".json");
    let result = load_file(file.path());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().info.title, "Test");
}

#[test]
fn load_file_unknown_extension_parses_yaml() {
    let file = write_temp_file(MINIMAL_YAML, ".txt");
    let result = load_file(file.path());
    assert!(result.is_ok());
}

#[test]
fn load_file_unknown_extension_parses_json() {
    let file = write_temp_file(MINIMAL_JSON, ".txt");
    let result = load_file(file.path());
    assert!(result.is_ok());
}

#[test]
fn load_file_missing_file() {
    let result = load_file(Path::new("/nonexistent/path/spec.yaml"));
    assert!(matches!(result, Err(LoadError::Io(_, _))));
}

#[test]
fn load_file_invalid_content() {
    let file = write_temp_file("not valid openapi", ".yaml");
    let result = load_file(file.path());
    assert!(matches!(result, Err(LoadError::Parse(_, _))));
}

#[test]
fn load_error_display_includes_path() {
    let result = load_file(Path::new("/some/path.yaml"));
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("/some/path.yaml"));
}

#[test]
fn parse_error_display() {
    let err = parse_content("invalid", Format::Json).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("invalid JSON"));
}
