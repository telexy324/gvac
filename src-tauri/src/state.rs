use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use chrono::{DateTime, Utc};
use ssh2::Session;

use crate::models::{ConnectRequest, SessionInfo};

pub struct SshSession {
    pub info: SessionInfo,
    pub request: ConnectRequest,
    pub session: Session,
    pub op_lock: Arc<Mutex<()>>,
    pub _tcp: TcpStream,
}

pub struct TerminalSession {
    pub session_id: String,
    pub input_tx: Sender<TerminalWorkerCommand>,
    pub output_buffer: Arc<Mutex<Vec<u8>>>,
    pub last_error: Arc<Mutex<Option<String>>>,
    pub closed: Arc<AtomicBool>,
    pub join_handle: Option<JoinHandle<()>>,
    pub op_lock: Arc<Mutex<()>>,
}

pub enum TerminalWorkerCommand {
    Write(Vec<u8>),
    Resize { cols: u32, rows: u32 },
    Close,
}

#[derive(Clone, Default)]
pub struct ApiClientState {
    pub base_url: Option<String>,
    pub timeout_ms: Option<u64>,
    pub token: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Clone, Default)]
pub struct AppState {
    pub sessions: Arc<Mutex<HashMap<String, SshSession>>>,
    pub terminals: Arc<Mutex<HashMap<String, TerminalSession>>>,
    pub api: Arc<Mutex<ApiClientState>>,
}

pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

pub fn set_last_active(session: &mut SshSession) {
    session.info.last_active_at = now_utc();
}
