use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub domain: String,
    pub status: Option<u16>,
    pub latency_ms: u64,
    pub intercepted: bool,
    pub error: Option<String>,
}
