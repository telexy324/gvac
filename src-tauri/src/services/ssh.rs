use std::net::TcpStream;
use std::path::Path;

use ssh2::Session;
use uuid::Uuid;

use crate::errors::{AppError, AppResult};
use crate::models::{AuthMethod, ConnectRequest, SessionInfo};
use crate::state::{now_utc, SshSession};

pub fn connect_ssh(request: ConnectRequest) -> AppResult<SshSession> {
    if request.host.trim().is_empty() || request.username.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "host and username are required".to_string(),
        ));
    }

    let address = format!("{}:{}", request.host.trim(), request.port);
    let tcp = TcpStream::connect(address)?;
    tcp.set_nodelay(true)?;

    let mut session = Session::new()?;
    session.set_tcp_stream(tcp.try_clone()?);
    session.handshake()?;

    match request.auth {
        AuthMethod::None => {
            // Trigger "none" auth flow; for NoClientAuth servers this should authenticate directly.
            let _ = session.auth_methods(request.username.trim())?;
        }
        AuthMethod::Password { password } => {
            session.userauth_password(request.username.trim(), password.as_str())?;
        }
        AuthMethod::PrivateKey {
            private_key_path,
            passphrase,
        } => {
            session.userauth_pubkey_file(
                request.username.trim(),
                None,
                Path::new(private_key_path.trim()),
                passphrase.as_deref(),
            )?;
        }
    }

    if !session.authenticated() {
        return Err(AppError::AuthFailed);
    }

    session.set_keepalive(true, 30);

    let id = Uuid::new_v4().to_string();
    let connected_at = now_utc();
    let label = request
        .label
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| format!("{}@{}", request.username, request.host));

    Ok(SshSession {
        info: SessionInfo {
            id,
            label,
            host: request.host,
            port: request.port,
            username: request.username,
            connected_at,
            last_active_at: connected_at,
        },
        session,
        _tcp: tcp,
    })
}
