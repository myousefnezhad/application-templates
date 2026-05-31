use serde::{Deserialize, Serialize};
use tracing_subscriber::filter::LevelFilter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn into_tracing_level(&self) -> LevelFilter {
        match self {
            &LogLevel::Off => LevelFilter::OFF,
            &LogLevel::Error => LevelFilter::ERROR,
            &LogLevel::Warn => LevelFilter::WARN,
            &LogLevel::Info => LevelFilter::INFO,
            &LogLevel::Debug => LevelFilter::DEBUG,
            &LogLevel::Trace => LevelFilter::TRACE,
        }
    }
}

pub fn init_tracing(level: LogLevel) {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(level.into_tracing_level())
        .init();
}
