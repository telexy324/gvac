use std::fs::File;
use std::io::Write;
use std::path::Path;

use tauri::State;

use crate::errors::{state_lock_poisoned, AppError, AppResult};
use crate::models::SftpEntry;
use crate::state::{set_last_active, AppState};

#[tauri::command]
pub fn sftp_list_dir(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> AppResult<Vec<SftpEntry>> {
    let normalized = if path.trim().is_empty() {
        "."
    } else {
        path.trim()
    };

    let op_lock = {
        let sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
        let item = sessions.get(&session_id).ok_or(AppError::SessionNotFound)?;
        item.op_lock.clone()
    };
    let _op_guard = op_lock.lock().map_err(|_| state_lock_poisoned())?;

    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    let item = sessions
        .get_mut(&session_id)
        .ok_or(AppError::SessionNotFound)?;
    item.session.set_blocking(true);

    let sftp = item.session.sftp()?;
    let entries = sftp.readdir(Path::new(normalized))?;

    let mapped = entries
        .into_iter()
        .map(|(path_buf, stat)| {
            let name = path_buf
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap_or_default()
                .to_string();
            let path = path_buf.to_string_lossy().to_string();
            let permissions = stat.perm;
            let kind = permissions
                .map(|perm| match perm & 0o170000 {
                    0o040000 => "dir",
                    0o100000 => "file",
                    0o120000 => "symlink",
                    _ => "unknown",
                })
                .unwrap_or("unknown")
                .to_string();

            SftpEntry {
                name,
                path,
                kind,
                size: stat.size,
                permissions,
                modified_at: stat.mtime,
            }
        })
        .collect();

    set_last_active(item);

    Ok(mapped)
}

#[tauri::command]
pub fn sftp_upload(
    state: State<'_, AppState>,
    session_id: String,
    local_path: String,
    remote_path: String,
) -> AppResult<()> {
    if local_path.trim().is_empty() || remote_path.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "local_path and remote_path are required".to_string(),
        ));
    }

    let op_lock = {
        let sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
        let item = sessions.get(&session_id).ok_or(AppError::SessionNotFound)?;
        item.op_lock.clone()
    };
    let _op_guard = op_lock.lock().map_err(|_| state_lock_poisoned())?;

    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    let item = sessions
        .get_mut(&session_id)
        .ok_or(AppError::SessionNotFound)?;
    item.session.set_blocking(true);

    let mut local_file = File::open(local_path.trim())?;
    let sftp = item.session.sftp()?;
    let mut remote_file = sftp.create(Path::new(remote_path.trim()))?;
    std::io::copy(&mut local_file, &mut remote_file)?;
    remote_file.flush()?;

    set_last_active(item);

    Ok(())
}

#[tauri::command]
pub fn sftp_download(
    state: State<'_, AppState>,
    session_id: String,
    remote_path: String,
    local_path: String,
) -> AppResult<()> {
    if local_path.trim().is_empty() || remote_path.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "local_path and remote_path are required".to_string(),
        ));
    }

    let op_lock = {
        let sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
        let item = sessions.get(&session_id).ok_or(AppError::SessionNotFound)?;
        item.op_lock.clone()
    };
    let _op_guard = op_lock.lock().map_err(|_| state_lock_poisoned())?;

    let mut sessions = state.sessions.lock().map_err(|_| state_lock_poisoned())?;
    let item = sessions
        .get_mut(&session_id)
        .ok_or(AppError::SessionNotFound)?;
    item.session.set_blocking(true);

    let sftp = item.session.sftp()?;
    let mut remote_file = sftp.open(Path::new(remote_path.trim()))?;
    let mut local_file = File::create(local_path.trim())?;
    std::io::copy(&mut remote_file, &mut local_file)?;
    local_file.flush()?;

    set_last_active(item);

    Ok(())
}
