use std::io::Read;

use tauri::State;

use crate::errors::{state_lock_poisoned, AppError, AppResult};
use crate::models::{CommandOutput, KeepaliveStatus};
use crate::state::{set_last_active, AppState};

#[tauri::command]
pub fn run_command(
    state: State<'_, AppState>,
    session_id: String,
    command: String,
) -> AppResult<CommandOutput> {
    if command.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "command cannot be empty".to_string(),
        ));
    }

    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    let item = sessions
        .get_mut(&session_id)
        .ok_or(AppError::SessionNotFound)?;
    item.session.set_blocking(true);

    let mut channel = item.session.channel_session()?;
    channel.exec(command.as_str())?;

    let mut stdout = String::new();
    channel.read_to_string(&mut stdout)?;

    let mut stderr = String::new();
    channel.stderr().read_to_string(&mut stderr)?;

    channel.wait_close()?;
    let exit_code = channel.exit_status()?;

    set_last_active(item);

    Ok(CommandOutput {
        stdout,
        stderr,
        exit_code,
    })
}

#[tauri::command]
pub fn send_keepalive(
    state: State<'_, AppState>,
    session_id: String,
) -> AppResult<KeepaliveStatus> {
    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;

    let item = sessions
        .get_mut(&session_id)
        .ok_or(AppError::SessionNotFound)?;
    item.session.set_blocking(true);

    let seconds_to_next = item.session.keepalive_send()?;
    set_last_active(item);

    Ok(KeepaliveStatus { seconds_to_next })
}
