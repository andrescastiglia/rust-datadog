use crate::{apm_config::ApmConfig, logging_config::LoggingConfig};

/// Configuration settings for the client.
pub struct Config {
    /// Datadog apm service name
    service: String,
    /// Datadog apm environment
    environment: Option<String>,
    /// Datadog agent host/ip + port, defaults to `localhost:8196`.
    endpoint: String,
    /// Optional Logging Config to also set this tracer as the main logger
    logging_config: LoggingConfig,
    /// APM Config to set up APM Analytics (default is to disable)
    apm_config: ApmConfig,
    /// Turn on tracing
    enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            environment: None,
            endpoint: "http://localhost:8123/v0.3/traces".to_owned(),
            service: "default".to_owned(),
            logging_config: LoggingConfig::default(),
            apm_config: ApmConfig::default(),
            enabled: true,
        }
    }
}

impl Config {
    #[must_use]
    pub fn new(
        service: String,
        environment: Option<String>,
        endpoint: String,
        logging_config: LoggingConfig,
        apm_config: ApmConfig,
        enabled: bool,
    ) -> Self {
        Config {
            service,
            environment,
            endpoint,
            logging_config,
            apm_config,
            enabled,
        }
    }
    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }
    #[must_use]
    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
    #[must_use]
    pub fn logging_config(&self) -> &LoggingConfig {
        &self.logging_config
    }
    #[must_use]
    pub fn apm_config(&self) -> &ApmConfig {
        &self.apm_config
    }
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }
}
