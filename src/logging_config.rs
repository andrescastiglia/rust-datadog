use log::Level;

pub struct LoggingConfig {
    level: Level,
    time_format: String,
    mod_filter: Vec<String>,
    body_filter: Vec<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: Level::Trace,
            time_format: "%Y-%m-%d %H:%M:%S%z".to_owned(),
            mod_filter: vec!["hyper".to_owned(), "mime".to_owned()],
            body_filter: Vec::default(),
        }
    }
}

impl LoggingConfig {
    #[must_use]
    pub fn new(
        level: Level,
        time_format: String,
        mod_filter: Vec<String>,
        body_filter: Vec<String>,
    ) -> Self {
        LoggingConfig {
            level,
            time_format,
            mod_filter,
            body_filter,
        }
    }
    #[must_use]
    pub fn level(&self) -> Level {
        self.level
    }
    #[must_use]
    pub fn time_format(&self) -> &str {
        &self.time_format
    }
    #[must_use]
    pub fn mod_filter(&self) -> &[String] {
        &self.mod_filter
    }
    #[must_use]
    pub fn body_filter(&self) -> &[String] {
        &self.body_filter
    }
}
