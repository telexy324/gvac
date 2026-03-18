#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod diagnostics;
mod errors;
mod models;
mod services;
mod state;

use commands::{
    api_clear_auth, api_login, api_request, api_set_auth, api_set_base_url, close_session,
    close_terminal, create_session, duplicate_session, http_get, http_post_json, http_request,
    list_sessions, run_command, send_keepalive, sftp_download, sftp_list_dir, sftp_upload,
    start_terminal, terminal_read, terminal_resize, terminal_write,
};
use state::AppState;

fn main() {
    let log_dir = diagnostics::init();

    let app_result = tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            create_session,
            duplicate_session,
            list_sessions,
            close_session,
            run_command,
            send_keepalive,
            sftp_list_dir,
            sftp_upload,
            sftp_download,
            start_terminal,
            terminal_write,
            terminal_read,
            terminal_resize,
            close_terminal,
            http_request,
            http_get,
            http_post_json,
            api_set_base_url,
            api_set_auth,
            api_clear_auth,
            api_request,
            api_login
        ])
        .run(tauri::generate_context!());

    match app_result {
        Ok(_) => diagnostics::log_runtime(&log_dir, "app exited normally"),
        Err(err) => {
            diagnostics::log_runtime(&log_dir, &format!("fatal tauri runtime error: {err}"));
            panic!("error while running tauri app: {err}");
        }
    }
}
