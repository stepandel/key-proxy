use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Mutex;

const MAX_LOG: usize = 500;

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub domain: String,
    pub status: Option<u16>,
    pub latency_ms: u64,
    pub intercepted: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsSnapshot {
    pub requests_today: u64,
    pub errors_today: u64,
    pub recent: Vec<LogEntry>,
}

pub struct Stats {
    log: Mutex<VecDeque<LogEntry>>,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            log: Mutex::new(VecDeque::with_capacity(MAX_LOG)),
        }
    }

    pub fn record(&self, entry: LogEntry) {
        let mut g = self.log.lock().unwrap();
        if g.len() == MAX_LOG {
            g.pop_front();
        }
        g.push_back(entry);
    }

    pub fn snapshot(&self) -> StatsSnapshot {
        let g = self.log.lock().unwrap();
        let today = Utc::now().date_naive();
        let mut requests_today = 0u64;
        let mut errors_today = 0u64;
        for e in g.iter() {
            if e.timestamp.date_naive() == today {
                requests_today += 1;
                if e.error.is_some() || e.status.map(|s| s >= 400).unwrap_or(false) {
                    errors_today += 1;
                }
            }
        }
        let recent: Vec<LogEntry> = g.iter().rev().take(100).cloned().collect();
        StatsSnapshot {
            requests_today,
            errors_today,
            recent,
        }
    }
}
