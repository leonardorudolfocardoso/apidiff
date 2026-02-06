use openapiv3::OpenAPI;
use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub enum ParseError {
    Yaml(serde_yml::Error),
    Json(serde_json::Error),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Yaml(e) => write!(f, "invalid YAML: {e}"),
            ParseError::Json(e) => write!(f, "invalid JSON: {e}"),
        }
    }
}

type LoadErrorPath = String;

#[derive(Debug)]
pub enum LoadError {
    Io(LoadErrorPath, std::io::Error),
    Parse(LoadErrorPath, ParseError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(path, e) => write!(f, "failed to read '{path}': {e}"),
            LoadError::Parse(path, e) => write!(f, "failed to parse '{path}': {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

enum Format {
    Json,
    Yaml,
}

fn detect_format(path: &Path) -> Option<Format> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("json") => Some(Format::Json),
        Some("yaml" | "yml") => Some(Format::Yaml),
        _ => None,
    }
}

fn parse_content(content: &str, format: Format) -> Result<OpenAPI, ParseError> {
    match format {
        Format::Json => serde_json::from_str(content).map_err(ParseError::Json),
        Format::Yaml => serde_yml::from_str(content).map_err(ParseError::Yaml),
    }
}

fn parse_unknown_content(content: &str) -> Result<OpenAPI, ParseError> {
    parse_content(content, Format::Json).or_else(|_| parse_content(content, Format::Yaml))
}

fn read_file(path: &Path) -> Result<String, LoadError> {
    let path_str = path.display().to_string();
    std::fs::read_to_string(path).map_err(|e| LoadError::Io(path_str, e))
}

fn parse_content_auto(content: &str, format: Option<Format>) -> Result<OpenAPI, ParseError> {
    match format {
        Some(f) => parse_content(content, f),
        None => parse_unknown_content(content),
    }
}

pub fn load_file(path: &Path) -> Result<OpenAPI, LoadError> {
    let path_str = path.display().to_string();
    let content = read_file(path)?;
    let format = detect_format(path);

    parse_content_auto(&content, format).map_err(|e| LoadError::Parse(path_str, e))
}

#[cfg(test)]
mod tests;
