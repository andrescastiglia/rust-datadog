type TimeInNanos = i64;
type ThreadId = u32;
type TraceId = u64;
type SpanId = u64;

pub(crate) mod agent_client;
pub mod apm_config;
pub mod config;
pub mod datadog_tracing;
pub(crate) mod hashmap_visitor;
pub(crate) mod log_record;
pub mod logging_config;
pub(crate) mod new_span_data;
pub(crate) mod raw_span;
pub(crate) mod span;
pub(crate) mod span_collection;
pub(crate) mod span_storage;
pub(crate) mod sql_info;
pub(crate) mod trace_command;

#[inline]
const fn ll2tl(level: log::Level) -> tracing::Level {
    match level {
        log::Level::Error => tracing::Level::ERROR,
        log::Level::Warn => tracing::Level::WARN,
        log::Level::Info => tracing::Level::INFO,
        log::Level::Debug => tracing::Level::DEBUG,
        log::Level::Trace => tracing::Level::TRACE,
    }
}
