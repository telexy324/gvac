#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod errors;
mod models;
mod services;
mod state;

use commands::{
    close_session, close_terminal, create_session, http_get, http_post_json, http_request,
    list_sessions, run_command, send_keepalive, sftp_download, sftp_list_dir, sftp_upload,
    start_terminal, terminal_read, terminal_resize, terminal_write,
};
use state::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            create_session,
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
            http_post_json
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri app");
}
