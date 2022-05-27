use crate::ThreadId;
use chrono::{DateTime, Utc};
use log::Level;

pub struct LogRecord {
    thread_id: ThreadId,
    level: Level,
    time: DateTime<Utc>,
    msg_str: String,
    module: Option<String>,
}

impl LogRecord {
    pub fn new(thread_id: ThreadId, level: Level, msg_str: String, module: Option<String>) -> Self {
        LogRecord {
            thread_id,
            level,
            time: chrono::Utc::now(),
            msg_str,
            module,
        }
    }
    pub fn thread_id(&self) -> ThreadId {
        self.thread_id
    }
    pub fn level(&self) -> Level {
        self.level
    }
    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }
    pub fn msg_str(&self) -> &str {
        &self.msg_str
    }
    pub fn module(&self) -> Option<&str> {
        self.module.as_deref()
    }
}
