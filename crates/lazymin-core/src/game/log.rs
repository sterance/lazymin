use std::collections::VecDeque;

const MAX_LOG_ENTRIES: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub uptime_secs: f64,
    pub text: String,
}

pub fn push_log(log: &mut VecDeque<LogEntry>, uptime_secs: f64, text: impl Into<String>) {
    log.push_back(LogEntry {
        uptime_secs,
        text: text.into(),
    });

    while log.len() > MAX_LOG_ENTRIES {
        log.pop_front();
    }
}
