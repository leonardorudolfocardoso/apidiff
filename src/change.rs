use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Breaking,
    NonBreaking,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    Path(String),
    Operation { path: String, method: String },
}

#[derive(Debug, Clone)]
pub struct Change {
    pub severity: Severity,
    pub location: Location,
    pub message: String,
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.location {
            Location::Path(path) => write!(f, "{path} - {}", self.message),
            Location::Operation { method, path } => {
                write!(f, "{method} {path} - {}", self.message)
            }
        }
    }
}
