use std::sync::{OnceLock, RwLock};
use std::{
    fmt::{Debug, Display, Formatter},
    path::PathBuf,
};
use std::{
    fs::create_dir_all,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

#[cfg(not(target_arch = "wasm32"))]
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    sync::mpsc::sync_channel,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

#[cfg(not(target_arch = "wasm32"))]
use std::fs::{read_dir, remove_file};

use crate::{DateTime, Map};

// ---------------------------------------------------------- //
// ------------------ Global logging state ------------------ //
// ---------------------------------------------------------- //
static LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Info as u8);
static LOG_LEVEL_MODULES: OnceLock<RwLock<Map<String, LogLevel>>> = OnceLock::new();

// A separate logger init variable is used to allow relaxed ordering load
static LOGGER_IS_ON: AtomicBool = AtomicBool::new(false);
static LOGGER_INSTANCE: OnceLock<Logger> = OnceLock::new();

pub const LOG_DBG: bool = cfg!(feature = "log_dbg");
pub const LOG_TRC: bool = cfg!(feature = "log_trc");

// ---------------------------------------------------------- //
// ----------------- Log level definitions ------------------ //
// ---------------------------------------------------------- //

/// Describes possibles levels that can be logged.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum LogLevel {
    Error = 1,
    Warning = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl From<u8> for LogLevel {
    fn from(value: u8) -> Self {
        match value {
            0..=1 => LogLevel::Error,
            2 => LogLevel::Warning,
            3 => LogLevel::Info,
            4 => LogLevel::Debug,
            _ => LogLevel::Trace,
        }
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => f.write_str("ERROR"),
            LogLevel::Warning => f.write_str("WARNING"),
            LogLevel::Info => f.write_str("INFO"),
            LogLevel::Debug => f.write_str("DEBUG"),
            LogLevel::Trace => f.write_str("TRACE"),
        }
    }
}

// ---------------------------------------------------------- //
// ----------------------- Logging api ---------------------- //
// ---------------------------------------------------------- //

/// Enables logging. Logging works both on native and WASM.
#[inline]
pub fn set_logger_on(log_level: LogLevel) {
    LOGGER_INSTANCE.get_or_init(|| Logger::new());
    LOGGER_IS_ON.store(true, Ordering::Relaxed);
    set_log_level(log_level, None);
}

/// Disables logging. Logging works both on native and WASM.
#[inline]
pub fn set_logger_off() {
    LOGGER_IS_ON.store(false, Ordering::Relaxed);
}

/// Checks if logging is enabled.
#[inline]
pub fn is_logger_on() -> bool {
    LOGGER_IS_ON.load(Ordering::Relaxed)
}

/// Enables logging to a file in the specified directory.
/// Does not exist on WASM.
#[cfg(not(target_arch = "wasm32"))]
#[inline]
pub fn set_log_file_write_on(log_dir: PathBuf) {
    if let Some(logger) = LOGGER_INSTANCE.get() {
        create_dir_all(log_dir.clone()).unwrap_or_else(|_| panic!("Failed to create log directory {:?}", log_dir));
        logger.log_to_file.store(true, Ordering::Relaxed);
        logger.set_log_dir(log_dir);
    }
}

/// Disables logging to a file.
/// Does not exist on WASM.
#[cfg(not(target_arch = "wasm32"))]
#[inline]
pub fn set_log_file_write_off() {
    if let Some(logger) = LOGGER_INSTANCE.get() {
        logger.log_to_file.store(false, Ordering::Relaxed);
    }
}

/// Returns the current log level for the module. If no module is specified, the global log level is returned.
#[inline]
pub fn log_level(module: Option<&str>) -> LogLevel {
    let default_level = LOG_LEVEL.load(Ordering::Relaxed).into();
    let module_levels = LOG_LEVEL_MODULES
        .get_or_init(|| RwLock::new(Map::default()))
        .read()
        .unwrap();

    if module_levels.is_empty() {
        return default_level;
    }

    if let Some(module) = module {
        module_levels
            .iter()
            .find(|(key, _)| module.contains(key.as_str()))
            .map(|(_, v)| *v)
            .unwrap_or_else(|| default_level)
    } else {
        default_level
    }
}

/// Sets log level to the specific target.
/// An optional module can be specified, in which case only that module's logging is changed.
/// If no optional module is specified, all logging will be set to target level,
/// i.e. all module-specific log levels will be wiped
#[inline]
pub fn set_log_level(log_level: LogLevel, module: Option<&str>) {
    if let Some(module) = module {
        LOG_LEVEL_MODULES
            .get_or_init(|| RwLock::new(Map::default()))
            .write()
            .unwrap()
            .insert(module.to_owned(), log_level);
    } else {
        LOG_LEVEL.store(log_level as u8, Ordering::Relaxed);
        LOG_LEVEL_MODULES
            .get_or_init(|| RwLock::new(Map::default()))
            .write()
            .unwrap()
            .clear();
    }
}

/// Flushes all logs immediately to the output specified by the logger implementation
#[inline]
pub fn flush_logs() {
    if let Some(logger) = LOGGER_INSTANCE.get() {
        logger.flush_logs();
    }
}

// ---------------------------------------------------------- //
// --------------------- Logging macros --------------------- //
// ---------------------------------------------------------- //

/// The main logging function. Usually this should not be called directly, and logging macros should be used instead.
/// Does nothing if called before logger is initialized and turned on.
#[inline]
pub fn log(level: LogLevel, module: &str, msg: String) {
    if !LOGGER_IS_ON.load(Ordering::Relaxed) {
        return;
    }

    if let Some(logger) = LOGGER_INSTANCE.get() {
        logger.log(format!("{:?} | {} | {} | {}\n", DateTime::now(), level, module, msg));
    }
}

#[macro_export]
macro_rules! log_error {
    ($($items:expr_2021),+) => {
        let module = module_path!();
        if $crate::is_logger_on() && $crate::log_level(Some(module)) >= $crate::LogLevel::Error {
            $crate::log($crate::LogLevel::Error, module, format!($($items),+));
        }
    };
}
#[macro_export]
macro_rules! log_warn {
    ($($items:expr_2021),+) => {
        let module = module_path!();
        if $crate::is_logger_on() && $crate::log_level(Some(module)) >= $crate::LogLevel::Warning {
            $crate::log($crate::LogLevel::Warning, module, format!($($items),+));
        }
    };
}
#[macro_export]
macro_rules! log_info {
    ($($items:expr_2021),+) => {
        let module = module_path!();
        if $crate::is_logger_on() && $crate::log_level(Some(module)) >= $crate::LogLevel::Info {
            $crate::log($crate::LogLevel::Info, module, format!($($items),+));
        }
    };
}
#[macro_export]
macro_rules! log_dbg {
    ($($items:expr_2021),+) => {
        if $crate::LOG_DBG {
            let module = module_path!();
            if $crate::is_logger_on() && $crate::log_level(Some(module)) >= $crate::LogLevel::Debug {
                $crate::log($crate::LogLevel::Debug, module, format!($($items),+));
            }
        }
    };
}

#[macro_export]
macro_rules! log_trc {
    ($($items:expr_2021),+) => {
        if $crate::LOG_TRC {
            let module = module_path!();
            if $crate::is_logger_on() && $crate::log_level(Some(module)) >= $crate::LogLevel::Trace {
                $crate::log($crate::LogLevel::Trace, module, format!($($items),+));
            }
        }
    };
}

// ---------------------------------------------------------- //
// ----------------- Logger implementation ------------------ //
// ---------------------------------------------------------- //

#[cfg(not(target_arch = "wasm32"))]
enum LogMessage {
    Log(String),
    SetLogDir(PathBuf),
    Flush(std::sync::mpsc::SyncSender<()>),
}

#[cfg(not(target_arch = "wasm32"))]
pub struct Logger {
    log_to_file: AtomicBool,
    sender: Sender<LogMessage>,
    _handle: JoinHandle<()>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Logger {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let handle = thread::spawn(move || {
            Self::background_thread(receiver);
        });

        Self {
            log_to_file: AtomicBool::new(false),
            sender,
            _handle: handle,
        }
    }

    pub fn log(&self, log_line: String) {
        print!("{}", log_line);
        self.sender.send(LogMessage::Log(log_line)).unwrap();
    }

    pub fn flush_logs(&self) {
        let (flush_sender, flush_receiver) = sync_channel(0);
        self.sender.send(LogMessage::Flush(flush_sender)).unwrap();
        // Block until flush is complete
        flush_receiver.recv().unwrap();
    }

    pub fn set_log_dir(&self, log_dir: PathBuf) {
        self.sender.send(LogMessage::SetLogDir(log_dir)).unwrap();
    }

    fn cleanup_old_log_files(log_dir: &PathBuf, max_files: usize) {
        if let Ok(entries) = read_dir(log_dir) {
            let mut log_files: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    if let Some(filename) = entry.file_name().to_str() {
                        filename.starts_with("log_at_") && filename.ends_with(".log")
                    } else {
                        false
                    }
                })
                .collect();

            // Sort by filename (which contains timestamp) in descending order (newest first)
            log_files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

            // Delete all but the newest max_files
            for file_to_delete in log_files.iter().skip(max_files) {
                if let Err(e) = remove_file(file_to_delete.path()) {
                    eprintln!("Failed to delete old log file {:?}: {}", file_to_delete.path(), e);
                }
            }
        }
    }

    fn background_thread(receiver: Receiver<LogMessage>) {
        let mut file_writer: Option<BufWriter<File>> = None;

        while let Ok(message) = receiver.recv() {
            match message {
                LogMessage::Log(log_line) => {
                    if let Some(ref mut writer) = file_writer {
                        writer
                            .write_all(log_line.as_bytes())
                            .expect("Failed to write to log file");
                    }
                }
                LogMessage::SetLogDir(log_dir) => {
                    let timestamp = DateTime::now().format_iso8601().replace(":", "-");
                    let filename = format!("log_at_{}.log", timestamp);
                    let log_path = log_dir.join(filename);

                    match OpenOptions::new().create(true).append(true).open(&log_path) {
                        Ok(file) => {
                            file_writer = Some(BufWriter::new(file));
                            // Clean up old log files, keeping only the 10 newest
                            Self::cleanup_old_log_files(&log_dir, 10);
                        }
                        Err(e) => {
                            eprintln!("Failed to open log file {:?}: {}", log_path, e);
                        }
                    }
                }
                LogMessage::Flush(flush_complete_flag) => {
                    if let Some(ref mut writer) = file_writer {
                        let _ = writer.flush();
                    }
                    flush_complete_flag.send(()).unwrap();
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub struct Logger {}

#[cfg(target_arch = "wasm32")]
impl Logger {
    pub fn new() -> Self {
        Self {}
    }

    pub fn log(&self, log_line: String) {
        crate::web_sys::console::log_1(&log_line.into());
    }

    pub fn flush_logs(&self) {}
}

// ---------------------------------------------------------- //
// ------------------------- Tests -------------------------- //
// ---------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn log_line_formatting_works() {
        let time = DateTime::from_unix_timestamp_ms(1703607295628);
        let formatted = format!(
            "{:?} | {} | {} | {}\n",
            time,
            LogLevel::Info,
            module_path!(),
            "test message".to_owned()
        );
        assert_eq!(
            formatted,
            "2023-12-26T16:14:55.628Z | INFO | ion_common::util::log::tests | test message\n"
        );
    }

    #[test]
    fn setting_log_level_works() {
        set_log_level(LogLevel::Warning, None);
        assert_eq!(log_level(None), LogLevel::Warning);
        set_log_level(LogLevel::Trace, None);
        assert_eq!(log_level(None), LogLevel::Trace);
    }

    #[test]
    fn setting_log_level_per_module_works() {
        set_log_level(LogLevel::Warning, Some("test_module"));
        assert_eq!(log_level(Some("test_module")), LogLevel::Warning);
        set_log_level(LogLevel::Trace, Some("test_module"));
        assert_eq!(log_level(Some("test_module")), LogLevel::Trace);
    }
}
