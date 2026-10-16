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
    /// Returns the effective console log level, clamped to file_level if file logging is active.
    pub fn effective_console_level(&self) -> log::LevelFilter {
        if self.file_dir.is_some() && self.console_level > self.file_level {
            self.file_level
        } else {
            self.console_level
        }
    }

    /// Formats the console log level for display, indicating if it was clamped to file_level.
    pub fn console_level_display(&self) -> String {
        let effective = self.effective_console_level();
        if effective != self.console_level {
            format!(
                "{} (clamped from {})",
                level_filter_to_str(effective),
                level_filter_to_str(self.console_level)
            )
        } else {
            level_filter_to_str(effective).to_string()
        }
    }

    /// Formats the file log level for display in lowercase.
    pub fn file_level_display(&self) -> &'static str {
        level_filter_to_str(self.file_level)
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

/// Rotates existing log files by shifting backup generations:
/// - Deletes the oldest backup ({basename}.{max_backups}.log) if it exists.
/// - Renames {basename}.{i}.log to {basename}.{i+1}.log in descending order.
/// - Renames the active log ({basename}.log) to {basename}.1.log.
///
/// If max_backups is 0, no rotation is performed. Failures during rotation
/// are reported to stderr as warnings without terminating execution.
fn rotate_backups(config: &LogConfig) {
    if config.max_backups == 0 {
        return;
    }

    let dir = match config.file_dir {
        Some(ref d) => d,
        None => return,
    };

    let max = config.max_backups;
    let basename = &config.file_basename;

    // Remove the oldest backup file if it exists.
    let oldest = dir.join(format!("{}.{}.log", basename, max));
    if oldest.exists() {
        if let Err(e) = fs::remove_file(&oldest) {
            eprintln!(
                "Warning: Failed to remove oldest log file {}: {}",
                oldest.display(),
                e
            );
        }
    }

    // Shift intermediate backup files in descending order.
    for i in (1..max).rev() {
        let src = dir.join(format!("{}.{}.log", basename, i));
        let dst = dir.join(format!("{}.{}.log", basename, i + 1));
        if src.exists() {
            if let Err(e) = fs::rename(&src, &dst) {
                eprintln!(
                    "Warning: Failed to rename log file {} to {}: {}",
                    src.display(),
                    dst.display(),
                    e
                );
            }
        }
    }

    // Rotate current log file to the first backup generation.
    let current = dir.join(format!("{}.log", basename));
    if current.exists() {
        let dst = dir.join(format!("{}.1.log", basename));
        if let Err(e) = fs::rename(&current, &dst) {
            eprintln!(
                "Warning: Failed to rename current log file {} to {}: {}",
                current.display(),
                dst.display(),
                e
            );
        }
    }
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
        rotate_backups(config);

        let file_spec = flexi_logger::FileSpec::default()
            .directory(dir)
            .basename(&config.file_basename)
            .suppress_timestamp();

        let duplicate = match config.effective_console_level() {
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

/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    /// RAII guard to create and reliably clean up an isolated temporary test directory.
    struct TestDir {
        path: path::PathBuf,
    }

    impl TestDir {
        fn new(prefix: &str) -> Self {
            let unique = format!(
                "{}_{}_{}",
                prefix,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            );
            let path = std::env::temp_dir().join(unique);
            fs::create_dir_all(&path).expect("Failed to create temporary test directory");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_rotate_backups_disabled_when_zero() {
        let temp_dir = TestDir::new("ritsu_log_test_zero");
        let active_log = temp_dir.path.join("test-app.log");
        fs::write(&active_log, "active log content").unwrap();

        let config = LogConfig {
            file_dir: Some(temp_dir.path.clone()),
            file_basename: String::from("test-app"),
            max_backups: 0,
            ..Default::default()
        };

        rotate_backups(&config);

        // Active log should remain unchanged without creating backup files.
        assert!(active_log.exists());
        assert_eq!(
            fs::read_to_string(&active_log).unwrap(),
            "active log content"
        );
        assert!(!temp_dir.path.join("test-app.1.log").exists());
    }

    #[test]
    fn test_rotate_backups_standard_shift_and_purge() {
        let temp_dir = TestDir::new("ritsu_log_test_shift");

        // Prepare existing files: active log and backup generations 1 through 5.
        fs::write(temp_dir.path.join("app.log"), "current").unwrap();
        for i in 1..=5 {
            fs::write(
                temp_dir.path.join(format!("app.{}.log", i)),
                format!("gen{}", i),
            )
            .unwrap();
        }

        let config = LogConfig {
            file_dir: Some(temp_dir.path.clone()),
            file_basename: String::from("app"),
            max_backups: 5,
            ..Default::default()
        };

        rotate_backups(&config);

        // Active log should have been rotated to generation 1.
        assert!(!temp_dir.path.join("app.log").exists());
        assert_eq!(
            fs::read_to_string(temp_dir.path.join("app.1.log")).unwrap(),
            "current"
        );

        // Intermediate backup generations should have shifted by one.
        assert_eq!(
            fs::read_to_string(temp_dir.path.join("app.2.log")).unwrap(),
            "gen1"
        );
        assert_eq!(
            fs::read_to_string(temp_dir.path.join("app.3.log")).unwrap(),
            "gen2"
        );
        assert_eq!(
            fs::read_to_string(temp_dir.path.join("app.4.log")).unwrap(),
            "gen3"
        );
        assert_eq!(
            fs::read_to_string(temp_dir.path.join("app.5.log")).unwrap(),
            "gen4"
        );

        // Oldest generation (gen5) should have been deleted, and generation 6 must not exist.
        assert!(!temp_dir.path.join("app.6.log").exists());
    }

    #[test]
    fn test_rotate_backups_with_missing_generations() {
        let temp_dir = TestDir::new("ritsu_log_test_sparse");

        // Only app.log and app.2.log exist (app.1.log is missing).
        fs::write(temp_dir.path.join("app.log"), "current").unwrap();
        fs::write(temp_dir.path.join("app.2.log"), "gen2").unwrap();

        let config = LogConfig {
            file_dir: Some(temp_dir.path.clone()),
            file_basename: String::from("app"),
            max_backups: 5,
            ..Default::default()
        };

        rotate_backups(&config);

        assert!(!temp_dir.path.join("app.log").exists());
        assert_eq!(
            fs::read_to_string(temp_dir.path.join("app.1.log")).unwrap(),
            "current"
        );
        assert!(!temp_dir.path.join("app.2.log").exists());
        assert_eq!(
            fs::read_to_string(temp_dir.path.join("app.3.log")).unwrap(),
            "gen2"
        );
    }
}
