#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Breaking,
    NonBreaking,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub path: String,
    pub method: String,
}

#[derive(Debug, Clone)]
pub struct Change {
    pub severity: Severity,
    pub location: Location,
    pub message: String,
}
