use tauri::State;

use crate::errors::{state_lock_poisoned, AppError, AppResult};
use crate::models::{ConnectRequest, SessionInfo};
use crate::services::ssh::connect_ssh;
use crate::state::AppState;

#[tauri::command]
pub fn create_session(
    state: State<'_, AppState>,
    request: ConnectRequest,
) -> AppResult<SessionInfo> {
    let created = connect_ssh(request)?;
    let session_info = created.info.clone();

    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    sessions.insert(session_info.id.clone(), created);

    Ok(session_info)
}

#[tauri::command]
pub fn duplicate_session(state: State<'_, AppState>, session_id: String) -> AppResult<SessionInfo> {
    let request = {
        let sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
        let existing = sessions.get(&session_id).ok_or(AppError::SessionNotFound)?;
        existing.request.clone()
    };

    let created = connect_ssh(request)?;
    let session_info = created.info.clone();

    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    sessions.insert(session_info.id.clone(), created);

    Ok(session_info)
}

#[tauri::command]
pub fn list_sessions(state: State<'_, AppState>) -> AppResult<Vec<SessionInfo>> {
    let sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;

    let mut list: Vec<SessionInfo> = sessions.values().map(|item| item.info.clone()).collect();
    list.sort_by(|a, b| b.connected_at.cmp(&a.connected_at));
    Ok(list)
}

#[tauri::command]
pub fn close_session(state: State<'_, AppState>, session_id: String) -> AppResult<()> {
    {
        let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
        sessions
            .remove(&session_id)
            .ok_or(AppError::SessionNotFound)
            .map(|_| ())?;
    }

    let mut terminals = state.terminals.lock().map_err(|_| state_lock_poisoned())?;

    let keys: Vec<String> = terminals
        .iter()
        .filter(|(_, terminal)| terminal.session_id == session_id)
        .map(|(id, _)| id.clone())
        .collect();

    for terminal_id in keys {
        if let Some(mut terminal) = terminals.remove(&terminal_id) {
            let _ = terminal.channel.close();
            let _ = terminal.channel.wait_close();
        }
    }

    Ok(())
}
