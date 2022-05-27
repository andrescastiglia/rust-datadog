use crate::{log_record::LogRecord, new_span_data::NewSpanData, SpanId, ThreadId, TimeInNanos};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub enum TraceCommand {
    Log(LogRecord),
    NewSpan(TimeInNanos, NewSpanData),
    Enter(TimeInNanos, ThreadId, SpanId),
    Exit(TimeInNanos, SpanId),
    CloseSpan(TimeInNanos, SpanId),
    Event(
        TimeInNanos,
        ThreadId,
        HashMap<String, String>,
        DateTime<Utc>,
    ),
}
