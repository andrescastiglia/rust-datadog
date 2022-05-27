use crate::{apm_config::ApmConfig, config::Config, span::Span, SpanId, TimeInNanos, TraceId};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

const SAMPLING_PRIORITY_KEY: &str = "_sampling_priority_v1";
const ANALYTICS_SAMPLE_RATE_KEY: &str = "_dd1.sr.eausr";
const _SAMPLE_RATE_METRIC_KEY: &str = "_sample_rate";
const _SAMPLING_AGENT_DECISION: &str = "_dd.agent_psr";
const _SAMPLING_RULE_DECISION: &str = "_dd.rule_psr";
const _SAMPLING_LIMIT_DECISION: &str = "_dd.limit_psr";

#[derive(Serialize, PartialEq)]
pub struct RawSpan {
    service: String,
    name: String,
    resource: String,
    trace_id: TraceId,
    span_id: SpanId,
    parent_id: Option<SpanId>,
    start: TimeInNanos,
    duration: TimeInNanos,
    error: i32,
    meta: HashMap<String, String>,
    metrics: HashMap<String, f64>,
    r#type: String,
}

impl RawSpan {
    pub fn from(span: &Span, config: &Arc<Config>) -> RawSpan {
        let http_enabled = span.tags().contains_key("http.url");
        let is_error = span.tags().contains_key("error.message");
        RawSpan {
            service: config.service().to_owned(),
            trace_id: span.trace_id(),
            span_id: span.id(),
            name: span.name().to_owned(),
            resource: span.resource().to_owned(),
            parent_id: span.parent_id(),
            start: span.start().timestamp_nanos(),
            duration: span.duration().num_nanoseconds().unwrap_or_default(),
            error: if is_error { 1 } else { 0 },
            r#type: if http_enabled { "custom" } else { "web" }.to_owned(),
            meta: Self::fill_meta(span, config.environment()),
            metrics: Self::fill_metrics(config.apm_config()),
        }
    }

    fn fill_meta(span: &Span, environment: Option<&str>) -> HashMap<String, String> {
        let mut meta = HashMap::with_capacity(span.tags().len() + 4);

        if let Some(environment) = environment {
            meta.insert("env".to_owned(), environment.to_owned());
        }

        if let Some(sql) = span.sql() {
            meta.insert("sql.query".to_owned(), sql.query().to_owned());
            meta.insert("sql.rows".to_owned(), sql.rows().to_owned());
            meta.insert("sql.db".to_owned(), sql.db().to_owned());
        }

        span.tags().iter().for_each(|(key, value)| {
            meta.insert(key.clone(), value.clone());
        });

        meta
    }

    fn fill_metrics(apm_config: &ApmConfig) -> HashMap<String, f64> {
        if apm_config.apm_enabled() {
            HashMap::from([
                (
                    SAMPLING_PRIORITY_KEY.to_owned(),
                    apm_config.sample_priority(),
                ),
                (
                    ANALYTICS_SAMPLE_RATE_KEY.to_owned(),
                    apm_config.sample_rate(),
                ),
            ])
        } else {
            HashMap::default()
        }
    }
}
