use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use ssh2::{Channel, Session};

use crate::models::SessionInfo;

pub struct SshSession {
    pub info: SessionInfo,
    pub session: Session,
    pub _tcp: TcpStream,
}

pub struct TerminalSession {
    pub session_id: String,
    pub channel: Channel,
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
