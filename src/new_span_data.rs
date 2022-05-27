use crate::{SpanId, TraceId};
use chrono::{DateTime, Utc};

pub struct NewSpanData {
    trace_id: TraceId,
    id: SpanId,
    name: String,
    resource: String,
    start: DateTime<Utc>,
}

impl NewSpanData {
    pub fn new(trace_id: TraceId, id: SpanId, name: String, resource: String) -> Self {
        NewSpanData {
            trace_id,
            id,
            name,
            resource,
            start: Utc::now(),
        }
    }
    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }
    pub fn id(&self) -> SpanId {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn resource(&self) -> &str {
        &self.resource
    }
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }
}
