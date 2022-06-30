use crate::{
    agent_client::AgentClient, config::Config, hashmap_visitor::HashMapVisitor,
    log_record::LogRecord, new_span_data::NewSpanData, span::Span, span_storage::SpanStorage,
    trace_command::TraceCommand, SpanId, ThreadId, TimeInNanos, TraceId,
};
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use log::{warn, Log, Record};
use rand::Rng;
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, AtomicU8, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, RwLock,
    },
};

static mut SAMPLING_RATE: Option<f64> = None;

lazy_static! {
    static ref UNIQUEID_COUNTER: AtomicU8 = AtomicU8::new(0);
    static ref THREAD_COUNTER: AtomicU32 = AtomicU32::new(0);
}

thread_local! {
    static THREAD_ID: ThreadId = THREAD_COUNTER.fetch_add(1, Ordering::Relaxed);
    static CURRENT_SPAN_ID: RwLock<RefCell<Option<SpanId>>> = RwLock::new(RefCell::new(None));
}

pub struct DatadogTracing {
    sender: Sender<TraceCommand>,
    level: log::Level,
    tracing_level: tracing::Level,
}

unsafe impl Sync for DatadogTracing {}

impl DatadogTracing {
    #[must_use]
    pub fn new(config: Config) -> Self {
        let config = Arc::new(config);

        let (sender, receiver) = mpsc::channel();
        {
            let config = Arc::clone(&config);
            let client = AgentClient::new(&config);

            std::thread::spawn(move || Self::trace_server_loop(&client, &receiver, &config));
        }

        // Only set the global sample rate once when the tracer is set as the global tracer.
        // This must be marked unsafe because we are overwriting a global, but it only gets done
        // once in a process's lifetime.
        unsafe {
            if SAMPLING_RATE.is_none() {
                SAMPLING_RATE = Some(config.apm_config().sample_rate());
            }
        }

        Self {
            sender,
            level: config.logging_config().level(),
            tracing_level: crate::ll2tl(config.logging_config().level()),
        }
    }
    pub fn init(config: Config) {
        tracing::subscriber::set_global_default(Self::new(config)).unwrap_or_else(|_| {
            warn!(
                "Global subscriber has already been set!  \
                           This should only be set once in the executable."
            );
        });
    }
    #[must_use]
    pub fn get_global_sampling_rate() -> f64 {
        unsafe { SAMPLING_RATE.unwrap_or_default() }
    }

    fn send_log(&self, record: LogRecord) {
        self.sender.send(TraceCommand::Log(record)).ok();
    }

    fn send_new_span(&self, nanos: TimeInNanos, span: NewSpanData) {
        self.sender.send(TraceCommand::NewSpan(nanos, span)).ok();
    }

    fn send_enter_span(&self, nanos: TimeInNanos, thread_id: ThreadId, id: SpanId) {
        self.sender
            .send(TraceCommand::Enter(nanos, thread_id, id))
            .ok();
    }

    fn send_exit_span(&self, nanos: TimeInNanos, id: SpanId) {
        self.sender.send(TraceCommand::Exit(nanos, id)).ok();
    }

    fn send_close_span(&self, nanos: TimeInNanos, span_id: SpanId) {
        self.sender
            .send(TraceCommand::CloseSpan(nanos, span_id))
            .ok();
    }

    fn send_event(
        &self,
        nanos: TimeInNanos,
        thread_id: ThreadId,
        event: HashMap<String, String>,
        time: DateTime<Utc>,
    ) {
        self.sender
            .send(TraceCommand::Event(nanos, thread_id, event, time))
            .ok();
    }

    fn trace_server_loop(
        client: &AgentClient,
        buffer_receiver: &Receiver<TraceCommand>,
        config: &Arc<Config>,
    ) {
        let mut storage = SpanStorage::default();

        loop {
            match buffer_receiver.recv() {
                Ok(TraceCommand::Log(record)) => {
                    let config = config.logging_config();

                    let skip = record
                        .module()
                        .map(|module| {
                            config
                                .mod_filter()
                                .iter()
                                .any(|filter| module.contains(filter))
                        })
                        .unwrap_or_default();

                    let body_skip = config
                        .body_filter()
                        .iter()
                        .any(|filter| record.msg_str().contains(filter));

                    if !skip && !body_skip {
                        match storage
                            .get_trace_id_for_thread(record.thread_id())
                            .and_then(|tr_id| {
                                storage.current_span_id(tr_id).map(|sp_id| (tr_id, sp_id))
                            }) {
                            Some((tr, sp)) => {
                                // Both trace and span are active on this thread
                                println!(
                                    "{time} {level} [trace-id:{traceid} span-id:{spanid}] [{module}] {body}",
                                    time = record.time().format(config.time_format()),
                                    traceid = tr,
                                    spanid = sp,
                                    level = record.level(),
                                    module = record.module().unwrap_or("-"),
                                    body = record.msg_str()
                                );
                            }
                            _ => {
                                // Both trace and span are not active on this thread
                                println!(
                                    "{time} {level} [{module}] {body}",
                                    time = record.time().format(config.time_format()),
                                    level = record.level(),
                                    module = record.module().unwrap_or("-"),
                                    body = record.msg_str()
                                );
                            }
                        }
                    }
                }
                Ok(TraceCommand::NewSpan(_nanos, data)) => {
                    storage.start_span(Span::from(data));
                }
                Ok(TraceCommand::Enter(_nanos, thread_id, span_id)) => {
                    storage.enter_span(thread_id, span_id);
                }
                Ok(TraceCommand::Exit(_nanos, span_id)) => {
                    storage.exit_span(span_id);
                }
                Ok(TraceCommand::Event(_nanos, thread_id, mut event, time)) => {
                    // Events are only valid if the trace_id flag is set
                    // Send trace specified the trace to send, so use that instead of the thread's
                    // current trace.
                    if let Some(send_trace_id) = event.remove("send_trace").map_or_else(
                        || storage.get_trace_id_for_thread(thread_id),
                        |t| t.parse::<TraceId>().ok(),
                    ) {
                        let send_vec = storage.drain_completed(send_trace_id, time);
                        // Thread has ended this trace.  Until it enters a new span, it
                        // is not in a trace.
                        storage.remove_current_trace(send_trace_id);
                        if !send_vec.is_empty() {
                            client.send(send_vec);
                        }
                    }
                    // Tag events only work inside a trace, so get the trace from the thread.
                    // No trace means no tagging.
                    if let Some(trace_id) = storage.get_trace_id_for_thread(thread_id) {
                        if let Some(type_event) = event.remove("error.etype") {
                            storage.span_record_tag(trace_id, "error.type".to_string(), type_event);
                        }
                        event
                            .into_iter()
                            .for_each(|(key, value)| storage.span_record_tag(trace_id, key, value));
                    }
                }
                Ok(TraceCommand::CloseSpan(nanos, span_id)) => {
                    storage.end_span(nanos, span_id);
                }
                Err(_) => {
                    return;
                }
            }
        }
    }

    fn get_thread_id() -> ThreadId {
        THREAD_ID.with(|id| *id)
    }

    #[cfg(test)]
    fn get_current_span_id() -> Option<SpanId> {
        CURRENT_SPAN_ID.with(|id| *id.read().unwrap().borrow())
    }

    fn set_current_span_id(new_id: Option<SpanId>) {
        CURRENT_SPAN_ID.with(|id| {
            id.write().unwrap().replace(new_id);
        });
    }
}

impl tracing::Subscriber for DatadogTracing {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.level().le(&self.tracing_level)
    }

    fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        let nanos = Utc::now().timestamp_nanos();
        let mut new_span_visitor = HashMapVisitor::default();
        span.record(&mut new_span_visitor);
        let trace_id = new_span_visitor
            .remove("trace_id")
            .and_then(|s| s.parse::<TraceId>().ok())
            .unwrap_or_else(|| {
                let mut rng = rand::thread_rng();
                rng.gen::<TraceId>()
            });
        let mut rng = rand::thread_rng();
        let span_id = rng.gen::<SpanId>();
        let new_span = NewSpanData::new(
            trace_id,
            span_id,
            span.metadata().name().to_owned(),
            span.metadata().target().to_owned(),
        );
        self.send_new_span(nanos, new_span);
        tracing::span::Id::from_u64(span_id)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let nanos = Utc::now().timestamp_nanos();
        let thread_id = Self::get_thread_id();
        let mut new_evt_visitor = HashMapVisitor::default();
        event.record(&mut new_evt_visitor);

        self.send_event(nanos, thread_id, new_evt_visitor.take(), Utc::now());
    }

    fn enter(&self, span: &tracing::span::Id) {
        let nanos = Utc::now().timestamp_nanos();
        let thread_id = Self::get_thread_id();
        self.send_enter_span(nanos, thread_id, span.into_u64());
        Self::set_current_span_id(Some(span.into_u64()));
    }

    fn exit(&self, span: &tracing::span::Id) {
        let nanos = Utc::now().timestamp_nanos();
        self.send_exit_span(nanos, span.into_u64());
        Self::set_current_span_id(None);
    }

    fn try_close(&self, span: tracing::span::Id) -> bool {
        let nanos = Utc::now().timestamp_nanos();
        self.send_close_span(nanos, span.into_u64());
        false
    }
}

impl Log for DatadogTracing {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level().le(&self.level)
    }

    fn log(&self, record: &Record) {
        if record.level() <= self.level {
            let log_rec = LogRecord::new(
                Self::get_thread_id(),
                record.level(),
                format!("{}", record.args()),
                record.module_path().map(ToOwned::to_owned),
            );
            self.send_log(log_rec);
        }
    }

    fn flush(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rand::Rng;
    use tracing::{debug, event, info, span};

    #[ctor::ctor]
    fn init() {
        DatadogTracing::init(Config::default());
    }

    // Format
    // |                       6 bytes                       |      2 bytes    |
    // +--------+--------+--------+--------+--------+--------+--------+--------+
    // |     number of milliseconds since epoch (1970)       | static counter  |
    // +--------+--------+--------+--------+--------+--------+--------+--------+
    // 0        8        16       24       32       40       48       56       64
    //
    // This will hold up to the year 10,000 before it cycles.
    fn create_unique_id64() -> u64 {
        let now = Utc::now();
        let baseline = Utc.timestamp(0, 0);

        let millis_since_epoch =
            (now.signed_duration_since(baseline).num_milliseconds() << 16) as u64;
        let rand: u8 = rand::thread_rng().gen_range(0..255u8);
        millis_since_epoch
            + ((rand as u64) << 8)
            + UNIQUEID_COUNTER.fetch_add(1, Ordering::Relaxed) as u64
    }

    fn long_call(trace_id: TraceId) {
        let span = span!(tracing::Level::INFO, "long_call", trace_id = trace_id);
        let _e = span.enter();
        debug!("Waiting on I/O {}", trace_id);
        sleep_call(trace_id);
        info!("I/O Finished {}", trace_id);
    }

    fn sleep_call(trace_id: TraceId) {
        let span = span!(tracing::Level::INFO, "sleep_call", trace_id = trace_id);
        let _e = span.enter();
        debug!("Long call {}", trace_id);
        debug!(
            "Current thread ID/span ID: {}/{:?}",
            DatadogTracing::get_thread_id(),
            DatadogTracing::get_current_span_id()
        );
        std::thread::sleep(std::time::Duration::from_millis(2000));
    }

    fn traced_func_no_send(trace_id: TraceId) {
        let span = span!(
            tracing::Level::INFO,
            "traced_func_no_send",
            trace_id = trace_id
        );
        let _e = span.enter();
        debug!(
            "Performing some function for id={}/{:?}",
            trace_id,
            DatadogTracing::get_current_span_id()
        );
        long_call(trace_id);
    }

    fn traced_http_func(trace_id: TraceId) {
        let span = span!(
            tracing::Level::INFO,
            "traced_http_func",
            trace_id = trace_id
        );
        let _e = span.enter();
        debug!(
            "Performing some function for id={}/{:?}",
            trace_id,
            DatadogTracing::get_current_span_id()
        );
        long_call(trace_id);
        event!(
            tracing::Level::INFO,
            http.url = "http://test.test/",
            http.status_code = "200",
            http.method = "GET"
        );
        event!(tracing::Level::INFO, send_trace = trace_id);
    }

    fn traced_error_func(trace_id: TraceId) {
        let span = span!(
            tracing::Level::INFO,
            "traced_error_func",
            trace_id = trace_id
        );
        let _e = span.enter();
        debug!(
            "Performing some function for id={}/{:?}",
            trace_id,
            DatadogTracing::get_current_span_id()
        );
        long_call(trace_id);
        event!(
            tracing::Level::ERROR,
            error.etype = "",
            error.message = "Test error"
        );
        event!(
            tracing::Level::ERROR,
            http.url = "http://test.test/",
            http.status_code = "400",
            http.method = "GET"
        );
        event!(
            tracing::Level::ERROR,
            custom_tag = "good",
            custom_tag2 = "test",
            send_trace = trace_id
        );
    }

    fn traced_error_func_single_event(trace_id: TraceId) {
        let span = span!(
            tracing::Level::INFO,
            "traced_error_func_single_event",
            trace_id = trace_id
        );
        let _e = span.enter();

        debug!(
            "Performing some function for id={}/{:?}",
            trace_id,
            DatadogTracing::get_current_span_id()
        );
        long_call(trace_id);
        event!(
            tracing::Level::ERROR,
            send_trace = trace_id,
            error.etype = "",
            error.message = "Test error",
            http.url = "http://test.test/",
            http.status_code = "400",
            http.method = "GET",
            custom_tag = "good",
            custom_tag2 = "test"
        );
    }

    #[test]
    fn test_trace_one_func_stack() {
        let trace_id = create_unique_id64();

        debug!(
            "Outside of span, this should be None: {:?}",
            DatadogTracing::get_current_span_id()
        );
        debug!(
            "Sampling rate is {}",
            DatadogTracing::get_global_sampling_rate()
        );

        let f1 = std::thread::spawn(move || {
            traced_func_no_send(trace_id);
            event!(tracing::Level::INFO, send_trace = trace_id);
        });

        debug!(
            "Same as before span, after span completes, this should be None: {:?}",
            DatadogTracing::get_current_span_id()
        );
        f1.join().unwrap();
        ::std::thread::sleep(::std::time::Duration::from_millis(1000));
    }

    #[test]
    fn test_parallel_two_threads_two_traces() {
        let trace_id1 = create_unique_id64();
        let trace_id2 = create_unique_id64();
        let f1 = std::thread::spawn(move || {
            traced_func_no_send(trace_id1);
            event!(tracing::Level::INFO, send_trace = trace_id1);
        });
        let f2 = std::thread::spawn(move || {
            traced_func_no_send(trace_id2);
            event!(tracing::Level::INFO, send_trace = trace_id2);
        });

        f1.join().unwrap();
        f2.join().unwrap();
        ::std::thread::sleep(::std::time::Duration::from_millis(1000));
    }

    #[test]
    fn test_parallel_two_threads_ten_traces() {
        let handlers = (0..10)
            .map(|i| {
                std::thread::spawn(move || {
                    let trace_id = create_unique_id64() + i;
                    traced_func_no_send(trace_id);
                    event!(tracing::Level::INFO, send_trace = trace_id);
                })
            })
            .collect::<Vec<_>>();

        for handler in handlers {
            handler.join().ok();
        }

        ::std::thread::sleep(::std::time::Duration::from_millis(1000));
    }

    #[test]
    fn test_error_span() {
        let trace_id = create_unique_id64();
        let f3 = std::thread::spawn(move || {
            traced_error_func(trace_id);
        });
        f3.join().unwrap();
        ::std::thread::sleep(::std::time::Duration::from_millis(1000));
    }

    #[test]
    fn test_error_span_as_single_event() {
        let trace_id = create_unique_id64();
        let f4 = std::thread::spawn(move || {
            traced_error_func_single_event(trace_id);
        });
        f4.join().unwrap();
        ::std::thread::sleep(::std::time::Duration::from_millis(1000));
    }

    #[test]
    fn test_two_funcs_in_one_span() {
        let trace_id = create_unique_id64();
        let f5 = std::thread::spawn(move || {
            traced_func_no_send(trace_id);
            traced_func_no_send(trace_id);
            // Send both funcs under one parent span and one trace
            event!(tracing::Level::INFO, send_trace = trace_id);
        });
        f5.join().unwrap();
        ::std::thread::sleep(::std::time::Duration::from_millis(1000));
    }

    #[test]
    fn test_one_thread_two_funcs_serial_two_traces() {
        let trace_id1 = create_unique_id64();
        let trace_id2 = create_unique_id64();
        let f7 = std::thread::spawn(move || {
            traced_func_no_send(trace_id1);
            event!(tracing::Level::INFO, send_trace = trace_id1);

            traced_func_no_send(trace_id2);
            event!(tracing::Level::INFO, send_trace = trace_id2);
        });
        f7.join().unwrap();
        ::std::thread::sleep(::std::time::Duration::from_millis(1000));
    }

    #[test]
    fn test_http_span() {
        let trace_id = create_unique_id64();
        let f3 = std::thread::spawn(move || {
            traced_http_func(trace_id);
        });
        f3.join().unwrap();
        ::std::thread::sleep(::std::time::Duration::from_millis(1000));
    }
}
