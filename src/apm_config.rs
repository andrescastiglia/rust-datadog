#[derive(Default)]
pub struct ApmConfig {
    apm_enabled: bool,
    sample_priority: f64,
    sample_rate: f64,
}

impl ApmConfig {
    #[must_use]
    pub fn new(apm_enabled: bool, sample_priority: f64, sample_rate: f64) -> Self {
        ApmConfig {
            apm_enabled,
            sample_priority,
            sample_rate,
        }
    }
    #[must_use]
    pub fn apm_enabled(&self) -> bool {
        self.apm_enabled
    }
    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }
    #[must_use]
    pub fn sample_priority(&self) -> f64 {
        self.sample_priority
    }
}
