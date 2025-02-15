use core::default::Default;

use clap::ValueEnum;
use log::LevelFilter;

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum LogLevel {
    #[default]
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Full,
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Off => LevelFilter::Off,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace | LogLevel::Full => LevelFilter::Trace,
        }
    }
}

#[derive(Clone)]
enum MessageEventKind {
    Info,
    Error,
}

#[derive(Clone)]
enum MessageEvent {
    Message(MessageEventKind, String),
    Flush,
}

#[cfg(feature = "const_logger")]
pub use const_logger::*;

#[cfg(feature = "const_logger")]
mod const_logger;

#[cfg(not(feature = "const_logger"))]
pub use dyn_logger::*;

#[cfg(not(feature = "const_logger"))]
mod dyn_logger;
