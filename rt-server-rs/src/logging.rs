// Copyright 2026 Naoyuki Kawano
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// =============================================================================
//!
//! Flexible asynchronous logging engine.
//!

use std::fs;
use std::io::Write;
use std::path;

/* -------------------------------------------------------------------------- */

/// Configuration for the logging subsystem.
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Console log level (duplicated to stderr).
    pub console_level: log::LevelFilter,

    /// Directory for log files. If None, file logging is disabled.
    pub file_dir: Option<path::PathBuf>,

    /// Base name of the log file without extension.
    pub file_basename: String,

    /// File log level.
    pub file_level: log::LevelFilter,

    /// Maximum number of backup log files for startup rotation (0 to disable).
    #[allow(dead_code)]
    pub max_backups: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            console_level: log::LevelFilter::Info,
            file_dir: None,
            file_basename: String::from("app"),
            file_level: log::LevelFilter::Debug,
            max_backups: 5,
        }
    }
}

impl LogConfig {
    /// Clamps the console log level to the file log level if file logging is enabled
    /// and the console log level is more verbose.
    pub fn clamp_console_level(&mut self) {
        if self.file_dir.is_some() && self.console_level > self.file_level {
            self.console_level = self.file_level;
        }
    }
}

/* -------------------------------------------------------------------------- */

/// Parses a log level string into a LevelFilter (case-insensitive).
pub fn parse_level(s: &str) -> Result<log::LevelFilter, String> {
    match s.trim().to_lowercase().as_str() {
        "off" => Ok(log::LevelFilter::Off),
        "error" => Ok(log::LevelFilter::Error),
        "warn" => Ok(log::LevelFilter::Warn),
        "info" => Ok(log::LevelFilter::Info),
        "debug" => Ok(log::LevelFilter::Debug),
        "trace" => Ok(log::LevelFilter::Trace),
        _ => Err(format!(
            "Invalid log level '{}'. Allowed: off, error, warn, info, debug, trace",
            s
        )),
    }
}

/// Converts a LevelFilter to its lowercase string representation.
fn level_filter_to_str(filter: log::LevelFilter) -> &'static str {
    match filter {
        log::LevelFilter::Off => "off",
        log::LevelFilter::Error => "error",
        log::LevelFilter::Warn => "warn",
        log::LevelFilter::Info => "info",
        log::LevelFilter::Debug => "debug",
        log::LevelFilter::Trace => "trace",
    }
}

/// Formats log records with standard microsecond timestamp layout:
/// YYYY/MM/DD HH:MM:SS.ffffff [LEVEL] Module - Message
fn format_log(
    w: &mut dyn Write,
    now: &mut flexi_logger::DeferredNow,
    record: &log::Record,
) -> Result<(), std::io::Error> {
    let target = record.target();
    let module = target.split("::").last().unwrap_or(target);
    write!(
        w,
        "{} [{:5}] {} - {}",
        now.format("%Y/%m/%d %H:%M:%S%.6f"),
        record.level(),
        module,
        record.args()
    )
}

/* -------------------------------------------------------------------------- */

/// Initializes flexi_logger based on the provided configuration.
///
/// When file logging is enabled:
/// - File writes are performed asynchronously using a dedicated background thread.
/// - Console output is duplicated to stderr.
/// - The console level is clamped if it is more verbose than the file level.
pub fn init(
    config: &LogConfig,
) -> Result<flexi_logger::LoggerHandle, flexi_logger::FlexiLoggerError> {
    if let Some(ref dir) = config.file_dir {
        fs::create_dir_all(dir)?;

        // Ensure console log level does not exceed file log level
        let mut console_level = config.console_level;
        if console_level > config.file_level {
            console_level = config.file_level;
        }

        let file_spec = flexi_logger::FileSpec::default()
            .directory(dir)
            .basename(&config.file_basename)
            .suppress_timestamp();

        let duplicate = match console_level {
            log::LevelFilter::Off => flexi_logger::Duplicate::None,
            log::LevelFilter::Error => flexi_logger::Duplicate::Error,
            log::LevelFilter::Warn => flexi_logger::Duplicate::Warn,
            log::LevelFilter::Info => flexi_logger::Duplicate::Info,
            log::LevelFilter::Debug => flexi_logger::Duplicate::Debug,
            log::LevelFilter::Trace => flexi_logger::Duplicate::Trace,
        };

        flexi_logger::Logger::try_with_str(level_filter_to_str(config.file_level))?
            .format(format_log)
            .log_to_file(file_spec)
            .write_mode(flexi_logger::WriteMode::Async)
            .duplicate_to_stderr(duplicate)
            .start()
    } else {
        flexi_logger::Logger::try_with_str(level_filter_to_str(config.console_level))?
            .format(format_log)
            .log_to_stderr()
            .start()
    }
}
