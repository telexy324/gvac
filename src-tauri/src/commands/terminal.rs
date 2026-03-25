use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ssh2::Channel;
use tauri::State;
use uuid::Uuid;

use crate::diagnostics;
use crate::errors::{state_lock_poisoned, AppError, AppResult};
use crate::models::TerminalStartResult;
use crate::state::{set_last_active, AppState, TerminalSession, TerminalWorkerCommand};

const MAX_READ_BYTES_PER_TICK: usize = 64 * 1024;
const MAX_BUFFER_BYTES: usize = 512 * 1024;
const WORKER_IDLE_SLEEP: Duration = Duration::from_millis(12);
const WRITE_RETRY_SLEEP: Duration = Duration::from_millis(2);

type TerminalContext = (
    String,
    Arc<Mutex<()>>,
    Sender<TerminalWorkerCommand>,
    Arc<Mutex<Vec<u8>>>,
    Arc<Mutex<Option<String>>>,
    Arc<AtomicBool>,
);

fn session_op_lock(state: &State<'_, AppState>, session_id: &str) -> AppResult<Arc<Mutex<()>>> {
    let sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    let item = sessions.get(session_id).ok_or(AppError::SessionNotFound)?;
    Ok(item.op_lock.clone())
}

fn terminal_context(state: &State<'_, AppState>, terminal_id: &str) -> AppResult<TerminalContext> {
    let terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;
    let terminal = terminals
        .get(terminal_id)
        .ok_or(AppError::TerminalNotFound)?;
    Ok((
        terminal.session_id.clone(),
        terminal.op_lock.clone(),
        terminal.input_tx.clone(),
        terminal.output_buffer.clone(),
        terminal.last_error.clone(),
        terminal.closed.clone(),
    ))
}

fn append_buffer(buffer: &mut Vec<u8>, chunk: &[u8]) {
    if chunk.is_empty() {
        return;
    }
    buffer.extend_from_slice(chunk);
    if buffer.len() > MAX_BUFFER_BYTES {
        let extra = buffer.len() - MAX_BUFFER_BYTES;
        buffer.drain(..extra);
    }
}

fn flush_nonblocking(channel: &mut Channel) -> std::io::Result<()> {
    loop {
        match channel.flush() {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(WRITE_RETRY_SLEEP);
            }
            Err(err) => return Err(err),
        }
    }
}

fn write_all_nonblocking(channel: &mut Channel, data: &[u8]) -> std::io::Result<()> {
    let mut offset = 0;
    while offset < data.len() {
        match channel.write(&data[offset..]) {
            Ok(0) => thread::sleep(WRITE_RETRY_SLEEP),
            Ok(n) => offset += n,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(WRITE_RETRY_SLEEP);
            }
            Err(err) => return Err(err),
        }
    }
    flush_nonblocking(channel)
}

fn read_nonblocking(channel: &mut Channel) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::<u8>::new();
    let mut buf = [0_u8; 4096];
    loop {
        if output.len() >= MAX_READ_BYTES_PER_TICK {
            break;
        }
        match channel.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => output.extend_from_slice(&buf[..n]),
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(err) => return Err(err),
        }
    }
    Ok(output)
}

fn set_last_error(last_error: &Arc<Mutex<Option<String>>>, msg: String) {
    if let Ok(mut slot) = last_error.lock() {
        *slot = Some(msg);
    }
}

fn terminal_worker_loop(
    session_id: String,
    terminal_id: String,
    session: ssh2::Session,
    op_lock: Arc<Mutex<()>>,
    mut channel: Channel,
    rx: Receiver<TerminalWorkerCommand>,
    output_buffer: Arc<Mutex<Vec<u8>>>,
    last_error: Arc<Mutex<Option<String>>>,
    closed: Arc<AtomicBool>,
) {
    diagnostics::log(&format!(
        "terminal_worker started session_id={session_id} terminal_id={terminal_id}"
    ));

    let mut running = true;
    while running {
        loop {
            match rx.try_recv() {
                Ok(TerminalWorkerCommand::Write(data)) => {
                    let op_guard = match op_lock.lock() {
                        Ok(guard) => guard,
                        Err(_) => {
                            set_last_error(&last_error, "state lock poisoned".to_string());
                            running = false;
                            break;
                        }
                    };
                    session.set_blocking(false);
                    let write_result = write_all_nonblocking(&mut channel, &data);
                    drop(op_guard);
                    if let Err(err) = write_result {
                        diagnostics::log(&format!(
                            "terminal_write worker failed terminal_id={terminal_id} err={err}"
                        ));
                        set_last_error(&last_error, format!("transport write: {err}"));
                        running = false;
                        break;
                    }
                }
                Ok(TerminalWorkerCommand::Resize { cols, rows }) => {
                    let op_guard = match op_lock.lock() {
                        Ok(guard) => guard,
                        Err(_) => {
                            set_last_error(&last_error, "state lock poisoned".to_string());
                            running = false;
                            break;
                        }
                    };
                    session.set_blocking(false);
                    let resize_result = channel.request_pty_size(cols.max(20), rows.max(5), None, None);
                    drop(op_guard);
                    if let Err(err) = resize_result {
                        diagnostics::log(&format!(
                            "terminal_resize worker failed terminal_id={terminal_id} err={err}"
                        ));
                        set_last_error(&last_error, format!("pty resize: {err}"));
                        running = false;
                        break;
                    }
                }
                Ok(TerminalWorkerCommand::Close) => {
                    running = false;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    running = false;
                    break;
                }
            }
        }

        if !running {
            break;
        }

        let read_result = {
            let op_guard = match op_lock.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    set_last_error(&last_error, "state lock poisoned".to_string());
                    break;
                }
            };
            session.set_blocking(false);
            let result = read_nonblocking(&mut channel);
            drop(op_guard);
            result
        };

        match read_result {
            Ok(chunk) => {
                if !chunk.is_empty() {
                    if let Ok(mut buf) = output_buffer.lock() {
                        append_buffer(&mut buf, &chunk);
                    }
                } else {
                    thread::sleep(WORKER_IDLE_SLEEP);
                }
            }
            Err(err) => {
                diagnostics::log(&format!(
                    "terminal_read worker failed terminal_id={terminal_id} err={err}"
                ));
                set_last_error(&last_error, format!("transport read: {err}"));
                running = false;
            }
        }
    }

    if let Ok(_guard) = op_lock.lock() {
        session.set_blocking(true);
        let _ = channel.close();
        let _ = channel.wait_close();
    }
    closed.store(true, Ordering::SeqCst);
    diagnostics::log(&format!(
        "terminal_worker exited session_id={session_id} terminal_id={terminal_id}"
    ));
}

#[tauri::command]
pub fn start_terminal(
    state: State<'_, AppState>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> AppResult<TerminalStartResult> {
    let op_lock = session_op_lock(&state, &session_id)?;
    let _op_guard = op_lock.lock().map_err(|_| state_lock_poisoned())?;

    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    let item = sessions
        .get_mut(&session_id)
        .ok_or(AppError::SessionNotFound)?;

    let session = item.session.clone();
    let mut channel = item.session.channel_session()?;
    let dimensions = Some((cols.max(20), rows.max(5), 0, 0));
    channel.request_pty("xterm-256color", None, dimensions)?;
    channel.shell()?;
    set_last_active(item);
    drop(sessions);
    drop(_op_guard);

    let terminal_id = Uuid::new_v4().to_string();
    let (input_tx, input_rx) = mpsc::channel::<TerminalWorkerCommand>();
    let output_buffer = Arc::new(Mutex::new(Vec::new()));
    let last_error = Arc::new(Mutex::new(None));
    let closed = Arc::new(AtomicBool::new(false));

    let thread_terminal_id = terminal_id.clone();
    let thread_session_id = session_id.clone();
    let thread_op_lock = op_lock.clone();
    let thread_output = output_buffer.clone();
    let thread_error = last_error.clone();
    let thread_closed = closed.clone();
    let join_handle = thread::Builder::new()
        .name(format!("pty-worker-{terminal_id}"))
        .spawn(move || {
            terminal_worker_loop(
                thread_session_id,
                thread_terminal_id,
                session,
                thread_op_lock,
                channel,
                input_rx,
                thread_output,
                thread_error,
                thread_closed,
            );
        })
        .map_err(AppError::Io)?;

    let terminal = TerminalSession {
        session_id: session_id.clone(),
        input_tx,
        output_buffer,
        last_error,
        closed,
        join_handle: Some(join_handle),
        op_lock: op_lock.clone(),
    };

    let mut terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;
    terminals.insert(terminal_id.clone(), terminal);
    diagnostics::log(&format!(
        "start_terminal session_id={session_id} terminal_id={terminal_id}"
    ));

    Ok(TerminalStartResult { terminal_id })
}

#[tauri::command]
pub fn terminal_write(
    state: State<'_, AppState>,
    terminal_id: String,
    data: String,
) -> AppResult<()> {
    let (session_id, _, input_tx, _, _, closed) = terminal_context(&state, &terminal_id)?;
    if closed.load(Ordering::SeqCst) {
        return Err(AppError::InvalidInput("terminal closed".to_string()));
    }
    input_tx
        .send(TerminalWorkerCommand::Write(data.into_bytes()))
        .map_err(|_| AppError::InvalidInput("terminal worker unavailable".to_string()))?;

    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    if let Some(item) = sessions.get_mut(&session_id) {
        set_last_active(item);
    }

    Ok(())
}

#[tauri::command]
pub fn terminal_read(state: State<'_, AppState>, terminal_id: String) -> AppResult<String> {
    let (_, _, _, output_buffer, last_error, closed) = terminal_context(&state, &terminal_id)?;

    let data = {
        let mut buffer = output_buffer.lock().map_err(|_| state_lock_poisoned())?;
        if buffer.is_empty() {
            Vec::new()
        } else {
            buffer.drain(..).collect::<Vec<u8>>()
        }
    };

    if data.is_empty() && closed.load(Ordering::SeqCst) {
        let message = last_error
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .unwrap_or_else(|| "terminal worker stopped".to_string());
        diagnostics::log(&format!(
            "terminal_read closed terminal_id={terminal_id} err={message}"
        ));
        return Err(AppError::Io(std::io::Error::other(message)));
    }

    Ok(String::from_utf8_lossy(&data).to_string())
}

#[tauri::command]
pub fn terminal_resize(
    state: State<'_, AppState>,
    terminal_id: String,
    cols: u32,
    rows: u32,
) -> AppResult<()> {
    let (_, _, input_tx, _, _, closed) = terminal_context(&state, &terminal_id)?;
    if closed.load(Ordering::SeqCst) {
        return Err(AppError::InvalidInput("terminal closed".to_string()));
    }
    input_tx
        .send(TerminalWorkerCommand::Resize { cols, rows })
        .map_err(|_| AppError::InvalidInput("terminal worker unavailable".to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn close_terminal(state: State<'_, AppState>, terminal_id: String) -> AppResult<()> {
    let mut terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;
    let mut terminal = terminals
        .remove(&terminal_id)
        .ok_or(AppError::TerminalNotFound)?;
    drop(terminals);

    let _ = terminal.input_tx.send(TerminalWorkerCommand::Close);
    if let Some(handle) = terminal.join_handle.take() {
        let _ = handle.join();
    }
    terminal.closed.store(true, Ordering::SeqCst);
    diagnostics::log(&format!("close_terminal terminal_id={terminal_id}"));
    Ok(())
}
