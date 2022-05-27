use crate::{span::Span, SpanId, TimeInNanos};
use chrono::{DateTime, Duration, Utc};
use std::collections::VecDeque;

pub struct SpanCollection {
    completed_spans: Vec<Span>,
    parent_span: Span,
    current_spans: VecDeque<Span>,
    entered_spans: VecDeque<SpanId>,
}

impl SpanCollection {
    pub fn new(parent_span: Span) -> Self {
        SpanCollection {
            completed_spans: vec![],
            parent_span,
            current_spans: VecDeque::default(),
            entered_spans: VecDeque::default(),
        }
    }

    // Open a span by inserting the span into the "current" span map by ID.
    pub fn start_span(&mut self, span: Span) {
        let parent_id = Some(
            self.current_span_id()
                .unwrap_or_else(|| self.parent_span.id()),
        );
        self.current_spans
            .push_back(Span::new_with_parent_id(parent_id, span));
    }

    // Move span to "completed" based on ID.
    pub fn end_span(&mut self, nanos: TimeInNanos, span_id: SpanId) {
        if let Some(i) = self.current_spans.iter().rposition(|i| i.id().eq(&span_id)) {
            if let Some(span) = self.current_spans.remove(i) {
                self.completed_spans.push(Span::new_with_duration(
                    Duration::nanoseconds(nanos - span.start().timestamp_nanos()),
                    span,
                ));
            }
        }
    }

    // Enter a span (mark it on stack)
    pub fn enter_span(&mut self, span_id: SpanId) {
        self.entered_spans.push_back(span_id);
    }

    // Exit a span (pop from stack)
    pub fn exit_span(&mut self, span_id: SpanId) {
        if let Some(i) = self.entered_spans.iter().rposition(|i| i.eq(&span_id)) {
            self.entered_spans.remove(i);
        }
    }

    /// Get the id, if present, of the most current span for this trace
    pub fn current_span_id(&self) -> Option<SpanId> {
        self.entered_spans.back().copied()
    }

    /// Add a tag
    pub fn add_tag(&mut self, key: String, value: String) {
        if let Some(span) = self.current_spans.back_mut() {
            span.add_tag(key.clone(), value.clone());
        }
        self.parent_span.add_tag(key, value);
    }

    /// Drain
    pub fn drain(&mut self, end_time: DateTime<Utc>) -> Vec<Span> {
        let parent_span = Span::new_with_duration(
            end_time.signed_duration_since(self.parent_span.start()),
            self.parent_span.clone(),
        );

        self.current_spans
            .drain(..)
            .collect::<Vec<Span>>()
            .into_iter()
            .for_each(|span| {
                self.completed_spans.push(Span::new_with_duration(
                    Utc::now().signed_duration_since(span.start()),
                    span,
                ));
            });

        let mut completed = self.completed_spans.drain(..).collect::<Vec<Span>>();

        completed.push(parent_span);

        completed
    }
}
