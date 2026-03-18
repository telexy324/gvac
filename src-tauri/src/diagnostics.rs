use std::any::Any;
use std::backtrace::Backtrace;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::panic::{self};
use std::path::PathBuf;
use std::sync::OnceLock;

use chrono::Utc;

const APP_DIR_NAME: &str = "gvac";
const LOGS_DIR_NAME: &str = "logs";
const APP_LOG_FILE: &str = "app.log";
const PANIC_LOG_FILE: &str = "panic.log";
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init() -> PathBuf {
    let log_dir = resolve_log_dir();
    if let Err(err) = create_dir_all(&log_dir) {
        eprintln!("failed to create log directory {:?}: {err}", log_dir);
    }

    append_line(
        &log_dir,
        APP_LOG_FILE,
        &format!("app starting at {}", Utc::now().to_rfc3339()),
    );
    install_panic_hook(log_dir.clone());
    let _ = LOG_DIR.set(log_dir.clone());
    log_dir
}

pub fn log_runtime(log_dir: &PathBuf, message: &str) {
    append_line(
        log_dir,
        APP_LOG_FILE,
        &format!("{} {message}", Utc::now().to_rfc3339()),
    );
}

pub fn log(message: &str) {
    let log_dir = LOG_DIR.get().cloned().unwrap_or_else(resolve_log_dir);
    append_line(
        &log_dir,
        APP_LOG_FILE,
        &format!("{} {message}", Utc::now().to_rfc3339()),
    );
}

fn resolve_log_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        return std::env::temp_dir().join(APP_DIR_NAME).join(LOGS_DIR_NAME);
    }

    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join(APP_DIR_NAME).join(LOGS_DIR_NAME);
    }

    std::env::temp_dir().join(APP_DIR_NAME).join(LOGS_DIR_NAME)
}

fn install_panic_hook(log_dir: PathBuf) {
    let previous_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        let timestamp = Utc::now().to_rfc3339();
        let location = info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown".to_string());
        let payload = panic_payload_to_string(info.payload());
        let thread_name = std::thread::current()
            .name()
            .map(ToString::to_string)
            .unwrap_or_else(|| "unnamed".to_string());
        let backtrace = Backtrace::force_capture();

        let body = format!(
            "{timestamp} panic on thread '{thread_name}' at {location}\nmessage: {payload}\nbacktrace:\n{backtrace}\n"
        );
        append_line(&log_dir, PANIC_LOG_FILE, &body);

        previous_hook(info);
    }));
}

fn panic_payload_to_string(payload: &(dyn Any + Send)) -> String {
    if let Some(value) = payload.downcast_ref::<&str>() {
        return (*value).to_string();
    }
    if let Some(value) = payload.downcast_ref::<String>() {
        return value.clone();
    }
    "non-string panic payload".to_string()
}

fn append_line(log_dir: &PathBuf, file_name: &str, line: &str) {
    let path = log_dir.join(file_name);
    let mut content = line.to_string();
    if !content.ends_with('\n') {
        content.push('\n');
    }

    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => {
            let _ = file.write_all(content.as_bytes());
        }
        Err(err) => {
            eprintln!("failed to open diagnostics log file {file_name}: {err}");
        }
    }
}
