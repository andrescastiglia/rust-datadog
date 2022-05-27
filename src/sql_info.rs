#[derive(Clone)]
pub struct SqlInfo {
    query: String,
    rows: String,
    db: String,
}

impl SqlInfo {
    pub fn query(&self) -> &str {
        &self.query
    }
    pub fn rows(&self) -> &str {
        &self.rows
    }
    pub fn db(&self) -> &str {
        &self.db
    }
}
