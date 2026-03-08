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
pub struct AppState {
    pub sessions: Arc<Mutex<HashMap<String, SshSession>>>,
    pub terminals: Arc<Mutex<HashMap<String, TerminalSession>>>,
}

pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

pub fn set_last_active(session: &mut SshSession) {
    session.info.last_active_at = now_utc();
}
