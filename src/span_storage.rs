use crate::{span::Span, span_collection::SpanCollection, SpanId, ThreadId, TimeInNanos, TraceId};
use chrono::{DateTime, Utc};
use rand::Rng;
use std::collections::HashMap;

#[derive(Default)]
pub struct SpanStorage {
    traces: HashMap<TraceId, SpanCollection>,
    spans_to_trace_id: HashMap<SpanId, TraceId>,
    current_trace_for_thread: HashMap<ThreadId, TraceId>,
    current_thread_for_trace: HashMap<TraceId, ThreadId>,
}

impl SpanStorage {
    // Either start a new trace with the span's trace ID (if there is no span already
    // pushed for that trace ID), or push the span on the "current" stack of spans for that
    // trace ID.  If "parent" is true, that means we need a parent span pushed for this to
    // represent the entire trace.
    pub fn start_span(&mut self, span: Span) {
        let trace_id = span.trace_id();
        self.spans_to_trace_id.insert(span.id(), span.trace_id());
        if let Some(ss) = self.traces.get_mut(&trace_id) {
            ss.start_span(span);
        } else {
            let mut rng = rand::thread_rng();
            let parent_span_id = rng.gen::<SpanId>();

            let parent_span = Span::new_with_id_name(
                parent_span_id,
                format!("{}-traceparent", trace_id),
                span.clone(),
            );

            let mut new_ss = SpanCollection::new(parent_span);
            new_ss.start_span(span);

            self.traces.insert(trace_id, new_ss);
        }
    }

    /// End a span and update the current "top of the stack"
    pub fn end_span(&mut self, nanos: TimeInNanos, span_id: SpanId) {
        if let Some(trace_id) = self.spans_to_trace_id.remove(&span_id) {
            if let Some(ss) = self.traces.get_mut(&trace_id) {
                ss.end_span(nanos, span_id);
            }
        }
    }

    /// Enter a span for trace, and keep track so that new spans get the correct parent.
    /// Keep track of which trace the current thread is in (for logging and events)
    pub fn enter_span(&mut self, thread_id: ThreadId, span_id: SpanId) {
        if let Some(trace_id) = self.spans_to_trace_id.get(&span_id) {
            if let Some(ss) = self.traces.get_mut(trace_id) {
                ss.enter_span(span_id);
            }

            self.current_trace_for_thread
                .insert(thread_id.to_owned(), trace_id.to_owned());

            self.current_thread_for_trace
                .insert(trace_id.to_owned(), thread_id.to_owned());
        }
    }

    /// Exit a span for trace, and keep track so that new spans get the correct parent
    pub fn exit_span(&mut self, span_id: SpanId) {
        if let Some(trace_id) = self.spans_to_trace_id.get(&span_id).copied() {
            if let Some(ss) = self.traces.get_mut(&trace_id) {
                ss.exit_span(span_id);
            }
            self.remove_current_trace(trace_id);
        }
    }

    /// Drain the span collection for this trace so we can send the trace through to Datadog,
    /// This effectively ends the trace.  Any new spans on this trace ID will have the same
    /// trace ID, but have a new parent span (and a new trace line in Datadog).
    pub fn drain_completed(&mut self, trace_id: TraceId, end: DateTime<Utc>) -> Vec<Span> {
        self.traces
            .remove(&trace_id)
            .map_or_else(Vec::default, |mut ss| ss.drain(end))
    }

    /// Record tag info onto a span
    pub fn span_record_tag(&mut self, trace_id: TraceId, key: String, value: String) {
        if let Some(ss) = self.traces.get_mut(&trace_id) {
            ss.add_tag(key, value);
        }
    }

    pub fn get_trace_id_for_thread(&self, thread_id: ThreadId) -> Option<TraceId> {
        self.current_trace_for_thread.get(&thread_id).copied()
    }

    pub fn remove_current_trace(&mut self, trace_id: TraceId) {
        if let Some(thread_id) = self.current_thread_for_trace.remove(&trace_id) {
            self.current_trace_for_thread.remove(&thread_id);
        }
    }

    /// Get the id, if present, of the most current span for the given trace
    pub fn current_span_id(&self, trace_id: TraceId) -> Option<SpanId> {
        self.traces
            .get(&trace_id)
            .and_then(SpanCollection::current_span_id)
    }
}
