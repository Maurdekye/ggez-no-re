use core::default::Default;
use std::{
    fs::{File, create_dir_all},
    io,
    path::PathBuf,
    sync::mpsc::{Receiver, Sender, channel},
    thread::{self, JoinHandle},
    time::SystemTime,
};

use std::io::Write as _;

use log::{Log, Record};

use crate::util::{ResultExtToIoError, SystemTimeExt};

use super::{LogLevel, MessageEvent, MessageEventKind};

pub struct Logger<const P: usize> {
    join_handle: Option<JoinHandle<()>>,
    message_sender: Option<Sender<MessageEvent>>,
    level_filter: log::LevelFilter,
    full_logs: bool,
    prefixes: [String; P],
}

impl<const P: usize> Logger<P> {
    pub fn install(self) -> Result<(), io::Error> {
        let level = self.level_filter.into();
        log::set_boxed_logger(Box::new(self))
            .map(|_| log::set_max_level(level))
            .to_ioerror()
    }

    pub fn new(
        log_path: Option<PathBuf>,
        level: LogLevel,
        prefixes: [impl Into<String>; P],
    ) -> Result<Logger<P>, io::Error> {
        let file = log_path
            .map(|mut path| {
                let now = SystemTime::now();
                path.push(now.strftime("%Y-%m-%d"));
                let _ = create_dir_all(&path);
                path.push(now.strftime("%H-%M-%S.log"));
                File::create(path)
            })
            .transpose()?;
        let (message_sender, message_receiver) = channel();
        let message_sender = Some(message_sender);
        let join_handle = thread::spawn(move || Logger::<P>::writer_thread(file, message_receiver));
        let join_handle = Some(join_handle);
        let level_filter = level.into();
        let full_logs = matches!(level, LogLevel::Full);
        let prefixes = prefixes.map(Into::into);
        Ok(Logger {
            join_handle,
            message_sender,
            level_filter,
            full_logs,
            prefixes,
        })
    }

    fn writer_thread(mut file: Option<File>, message_receiver: Receiver<MessageEvent>) {
        for message in message_receiver {
            match (message, &mut file) {
                (MessageEvent::Flush, Some(file)) => file.flush().unwrap(),
                (MessageEvent::Message(kind, text), file) => {
                    match kind {
                        MessageEventKind::Info => println!("{text}"),
                        MessageEventKind::Error => eprintln!("{text}"),
                    };
                    if let Some(file) = file {
                        let _ = writeln!(file, "{text}");
                    }
                }
                _ => {}
            }
        }
    }
}

impl<const P: usize> Log for Logger<P> {
    fn log(&self, record: &Record) {
        use log::Level::*;
        if !self.enabled(record.metadata()) {
            return;
        }
        let timestamp = SystemTime::now().strftime("%H:%M:%S%.3f");
        let log_str = format!(
            "[{}|{}|{}{}] {}",
            record.level(),
            timestamp,
            record.target(),
            record.line().map(|x| format!(":{x}")).unwrap_or_default(),
            record.args()
        );
        let _ = self
            .message_sender
            .as_ref()
            .unwrap()
            .send(MessageEvent::Message(
                match record.level() {
                    Error | Warn => MessageEventKind::Error,
                    Info | Debug | Trace => MessageEventKind::Info,
                },
                log_str,
            ));
    }

    fn enabled(&self, metadata: &log::Metadata) -> bool {
        (self.full_logs
            || self.prefixes.is_empty()
            || self
                .prefixes
                .iter()
                .any(|prefix| metadata.target().starts_with(prefix)))
            && metadata.level() <= self.level_filter
    }

    fn flush(&self) {
        let _ = self
            .message_sender
            .as_ref()
            .unwrap()
            .send(MessageEvent::Flush);
    }
}

impl<const P: usize> Drop for Logger<P> {
    fn drop(&mut self) {
        drop(self.message_sender.take());
        self.join_handle.take().unwrap().join().unwrap();
    }
}

pub struct LoggerBuilder<const P: usize> {
    pub path: Option<PathBuf>,
    pub level: LogLevel,
    pub prefixes: [String; P],
}

impl LoggerBuilder<0> {
    pub fn new() -> LoggerBuilder<0> {
        LoggerBuilder {
            path: Default::default(),
            level: Default::default(),
            prefixes: [],
        }
    }
}

impl<const P: usize> LoggerBuilder<P> {
    pub fn path_option(mut self, path: Option<PathBuf>) -> Self {
        self.path = path;
        self
    }

    pub fn path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    pub fn level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    pub fn prefix(self, prefix: impl Into<String>) -> LoggerBuilder<{ P + 1 }> {
        let LoggerBuilder {
            path,
            level,
            prefixes,
        } = self;
        let prefixes = prefixes
            .into_iter()
            .chain([prefix.into()])
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        LoggerBuilder {
            path,
            level,
            prefixes,
        }
    }

    pub fn build(self) -> Result<Logger<P>, io::Error> {
        Logger::new(self.path, self.level, self.prefixes)
    }

    pub fn install(self) -> Result<(), io::Error> {
        self.build()?.install()
    }
}
