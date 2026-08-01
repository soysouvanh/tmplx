// src/models.rs
#[derive(Debug, Clone)]
pub struct User {
    pub name: String,
    pub is_active: bool,
}

/// Fixture for the regression suite of §12, case 10 only
/// (`{% for %}` nested via a field of the enclosing element) — not
/// data from the reference view_data struct of §10, which doesn't need it.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Group {
    pub members: Vec<User>,
}
