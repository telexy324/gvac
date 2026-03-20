use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use tauri::State;
use uuid::Uuid;

use crate::diagnostics;
use crate::errors::{state_lock_poisoned, AppError, AppResult};
use crate::models::TerminalStartResult;
use crate::state::{set_last_active, AppState, TerminalSession};

const MAX_READ_BYTES_PER_POLL: usize = 64 * 1024;

fn session_op_lock(state: &State<'_, AppState>, session_id: &str) -> AppResult<Arc<Mutex<()>>> {
    let sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    let item = sessions.get(session_id).ok_or(AppError::SessionNotFound)?;
    Ok(item.op_lock.clone())
}

fn terminal_context(state: &State<'_, AppState>, terminal_id: &str) -> AppResult<(String, Arc<Mutex<()>>)> {
    let terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;
    let terminal = terminals.get(terminal_id).ok_or(AppError::TerminalNotFound)?;
    Ok((terminal.session_id.clone(), terminal.op_lock.clone()))
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

    let mut channel = item.session.channel_session()?;
    let dimensions = Some((cols.max(20), rows.max(5), 0, 0));
    channel.request_pty("xterm-256color", None, dimensions)?;
    channel.shell()?;
    set_last_active(item);

    let terminal_id = Uuid::new_v4().to_string();
    drop(sessions);

    let terminal = TerminalSession {
        session_id: session_id.clone(),
        channel,
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
    let (session_id, op_lock) = terminal_context(&state, &terminal_id)?;
    let _op_guard = op_lock.lock().map_err(|_| state_lock_poisoned())?;

    {
        let mut terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;
        let terminal = terminals
            .get_mut(&terminal_id)
            .ok_or(AppError::TerminalNotFound)?;
        if let Err(err) = terminal.channel.write_all(data.as_bytes()) {
            diagnostics::log(&format!(
                "terminal_write failed terminal_id={terminal_id} err={err}"
            ));
            return Err(AppError::Io(err));
        }
        if let Err(err) = terminal.channel.flush() {
            diagnostics::log(&format!(
                "terminal_write flush failed terminal_id={terminal_id} err={err}"
            ));
            return Err(AppError::Io(err));
        }
    }

    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    if let Some(item) = sessions.get_mut(&session_id) {
        set_last_active(item);
    }

    Ok(())
}

#[tauri::command]
pub fn terminal_read(state: State<'_, AppState>, terminal_id: String) -> AppResult<String> {
    let (session_id, op_lock) = terminal_context(&state, &terminal_id)?;
    let _op_guard = op_lock.lock().map_err(|_| state_lock_poisoned())?;

    {
        let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
        if let Some(item) = sessions.get_mut(&session_id) {
            item.session.set_blocking(false);
        }
    }

    let mut output = Vec::<u8>::new();
    {
        let mut terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;
        let terminal = terminals
            .get_mut(&terminal_id)
            .ok_or(AppError::TerminalNotFound)?;

        let mut buf = [0_u8; 4096];
        loop {
            if output.len() >= MAX_READ_BYTES_PER_POLL {
                break;
            }
            match terminal.channel.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(err) => {
                    diagnostics::log(&format!(
                        "terminal_read stdout failed terminal_id={terminal_id} err={err}"
                    ));
                    return Err(AppError::Io(err));
                }
            }
        }

        loop {
            if output.len() >= MAX_READ_BYTES_PER_POLL {
                break;
            }
            match terminal.channel.stderr().read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(err) => {
                    diagnostics::log(&format!(
                        "terminal_read stderr failed terminal_id={terminal_id} err={err}"
                    ));
                    return Err(AppError::Io(err));
                }
            }
        }
    }

    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    if let Some(item) = sessions.get_mut(&session_id) {
        item.session.set_blocking(true);
        set_last_active(item);
    }

    Ok(String::from_utf8_lossy(&output).to_string())
}

#[tauri::command]
pub fn terminal_resize(
    state: State<'_, AppState>,
    terminal_id: String,
    cols: u32,
    rows: u32,
) -> AppResult<()> {
    let (_, op_lock) = terminal_context(&state, &terminal_id)?;
    let _op_guard = op_lock.lock().map_err(|_| state_lock_poisoned())?;

    let mut terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;
    let terminal = terminals
        .get_mut(&terminal_id)
        .ok_or(AppError::TerminalNotFound)?;
    terminal
        .channel
        .request_pty_size(cols.max(20), rows.max(5), None, None)?;
    Ok(())
}

#[tauri::command]
pub fn close_terminal(state: State<'_, AppState>, terminal_id: String) -> AppResult<()> {
    let mut terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;
    let mut terminal = terminals
        .remove(&terminal_id)
        .ok_or(AppError::TerminalNotFound)?;
    drop(terminals);
    let _op_guard = terminal.op_lock.lock().map_err(|_| state_lock_poisoned())?;
    let _ = terminal.channel.close();
    let _ = terminal.channel.wait_close();
    diagnostics::log(&format!("close_terminal terminal_id={terminal_id}"));
    Ok(())
}
