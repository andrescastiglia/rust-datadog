use crate::{new_span_data::NewSpanData, sql_info::SqlInfo, SpanId, TraceId};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Span {
    id: SpanId,
    trace_id: TraceId,
    name: String,
    resource: String,
    parent_id: Option<SpanId>,
    start: DateTime<Utc>,
    duration: Duration,
    sql: Option<SqlInfo>,
    tags: HashMap<String, String>,
}

impl Span {
    pub fn new_with_id_name(id: SpanId, name: String, source: Span) -> Self {
        Span { id, name, ..source }
    }
    pub fn new_with_parent_id(parent_id: Option<SpanId>, source: Span) -> Self {
        Span {
            parent_id,
            ..source
        }
    }
    pub fn new_with_duration(duration: Duration, source: Span) -> Self {
        Span { duration, ..source }
    }

    pub fn id(&self) -> SpanId {
        self.id
    }
    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn resource(&self) -> &str {
        &self.resource
    }
    pub fn parent_id(&self) -> Option<SpanId> {
        self.parent_id
    }
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }
    pub fn duration(&self) -> Duration {
        self.duration
    }
    pub fn sql(&self) -> Option<&SqlInfo> {
        self.sql.as_ref()
    }
    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.insert(key, value);
    }
}

impl From<NewSpanData> for Span {
    fn from(new_span_data: NewSpanData) -> Self {
        Span {
            id: new_span_data.id(),
            trace_id: new_span_data.trace_id(),
            name: new_span_data.name().to_owned(),
            resource: new_span_data.resource().to_owned(),
            parent_id: None,
            start: new_span_data.start(),
            duration: Duration::seconds(0),
            sql: None,
            tags: HashMap::default(),
        }
    }
}
