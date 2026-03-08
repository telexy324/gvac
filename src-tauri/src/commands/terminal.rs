use std::io::{Read, Write};

use tauri::State;
use uuid::Uuid;

use crate::errors::{state_lock_poisoned, AppError, AppResult};
use crate::models::TerminalStartResult;
use crate::state::{set_last_active, AppState, TerminalSession};

#[tauri::command]
pub fn start_terminal(
    state: State<'_, AppState>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> AppResult<TerminalStartResult> {
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
        session_id,
        channel,
    };

    let mut terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;
    terminals.insert(terminal_id.clone(), terminal);

    Ok(TerminalStartResult { terminal_id })
}

#[tauri::command]
pub fn terminal_write(
    state: State<'_, AppState>,
    terminal_id: String,
    data: String,
) -> AppResult<()> {
    let session_id = {
        let mut terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;
        let terminal = terminals
            .get_mut(&terminal_id)
            .ok_or(AppError::TerminalNotFound)?;
        terminal.channel.write_all(data.as_bytes())?;
        terminal.channel.flush()?;
        terminal.session_id.clone()
    };

    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    if let Some(item) = sessions.get_mut(&session_id) {
        set_last_active(item);
    }

    Ok(())
}

#[tauri::command]
pub fn terminal_read(state: State<'_, AppState>, terminal_id: String) -> AppResult<String> {
    let session_id = {
        let mut terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;
        let terminal = terminals
            .get_mut(&terminal_id)
            .ok_or(AppError::TerminalNotFound)?;
        terminal.session_id.clone()
    };

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
            match terminal.channel.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(err) => return Err(AppError::Io(err)),
            }
        }

        loop {
            match terminal.channel.stderr().read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(err) => return Err(AppError::Io(err)),
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
    let _ = terminal.channel.close();
    let _ = terminal.channel.wait_close();
    Ok(())
}
