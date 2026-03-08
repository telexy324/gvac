use std::collections::HashMap;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::Method;
use tauri::State;

use crate::errors::{state_lock_poisoned, AppError, AppResult};
use crate::models::{ApiRequest, ApiResponse};
use crate::state::{ApiClientState, AppState};

const DEFAULT_TIMEOUT_MS: u64 = 60_000;

pub fn set_base_url(
    state: State<'_, AppState>,
    base_url: String,
    timeout_ms: Option<u64>,
) -> AppResult<()> {
    let normalized = normalize_base_url(base_url)?;
    let mut api = state.api.lock().map_err(|_| state_lock_poisoned())?;
    api.base_url = Some(normalized);
    api.timeout_ms = timeout_ms;
    Ok(())
}

pub fn set_auth(
    state: State<'_, AppState>,
    token: Option<String>,
    user_id: Option<String>,
) -> AppResult<()> {
    let mut api = state.api.lock().map_err(|_| state_lock_poisoned())?;
    api.token = token.filter(|v| !v.trim().is_empty());
    api.user_id = user_id.filter(|v| !v.trim().is_empty());
    Ok(())
}

pub fn clear_auth(state: State<'_, AppState>) -> AppResult<()> {
    let mut api = state.api.lock().map_err(|_| state_lock_poisoned())?;
    api.token = None;
    api.user_id = None;
    Ok(())
}

pub fn send_with_middleware(
    state: State<'_, AppState>,
    request: ApiRequest,
) -> AppResult<ApiResponse> {
    let snapshot = {
        let api = state.api.lock().map_err(|_| state_lock_poisoned())?;
        api.clone()
    };

    let client = build_client(request.timeout_ms, snapshot.timeout_ms)?;
    let method = parse_method(&request.method)?;
    let url = resolve_url(snapshot.base_url.as_deref(), &request.url)?;

    let mut builder = client.request(method, url);

    if let Some(query) = &request.query {
        builder = builder.query(query);
    }

    builder = apply_headers(
        builder,
        request.headers,
        &snapshot,
        request.use_auth.unwrap_or(true),
    );

    if let Some(data) = request.data {
        if let Some(s) = data.as_str() {
            builder = builder.body(s.to_string());
        } else {
            builder = builder.body(data.to_string());
        }
    }

    let response = builder.send()?;
    let mapped = map_response(response)?;

    if let Some(new_token) = mapped.new_token.clone() {
        let mut api = state.api.lock().map_err(|_| state_lock_poisoned())?;
        api.token = Some(new_token);
    }

    Ok(mapped)
}

fn normalize_base_url(base_url: String) -> AppResult<String> {
    let normalized = base_url.trim().trim_end_matches('/').to_string();
    if normalized.is_empty() {
        return Err(AppError::InvalidInput("base_url is required".to_string()));
    }
    Ok(normalized)
}

fn resolve_url(base_url: Option<&str>, request_url: &str) -> AppResult<String> {
    let path = request_url.trim();
    if path.is_empty() {
        return Err(AppError::InvalidInput("url is required".to_string()));
    }

    if path.starts_with("http://") || path.starts_with("https://") {
        return Ok(path.to_string());
    }

    match base_url {
        Some(base) if !base.trim().is_empty() => {
            if path.starts_with('/') {
                Ok(format!("{base}{path}"))
            } else {
                Ok(format!("{base}/{path}"))
            }
        }
        _ => Err(AppError::InvalidInput(
            "base_url is not set; provide an absolute url or call api_set_base_url first"
                .to_string(),
        )),
    }
}

fn build_client(timeout_ms: Option<u64>, default: Option<u64>) -> AppResult<Client> {
    let timeout =
        Duration::from_millis(timeout_ms.or(default).unwrap_or(DEFAULT_TIMEOUT_MS).max(1));
    Ok(Client::builder().timeout(timeout).build()?)
}

fn parse_method(method: &str) -> AppResult<Method> {
    Method::from_bytes(method.trim().as_bytes())
        .map_err(|_| AppError::InvalidInput(format!("invalid HTTP method: {method}")))
}

fn apply_headers(
    mut builder: reqwest::blocking::RequestBuilder,
    headers: Option<HashMap<String, String>>,
    state: &ApiClientState,
    use_auth: bool,
) -> reqwest::blocking::RequestBuilder {
    let mut merged = headers.unwrap_or_default();

    merged.insert("Content-Type".to_string(), "application/json".to_string());

    if use_auth {
        merged.insert(
            "x-token".to_string(),
            state.token.clone().unwrap_or_default(),
        );
        merged.insert(
            "x-user-id".to_string(),
            state.user_id.clone().unwrap_or_default(),
        );
    }

    for (key, value) in merged {
        builder = builder.header(key, value);
    }

    builder
}

fn map_response(response: reqwest::blocking::Response) -> AppResult<ApiResponse> {
    let status = response.status();

    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(key, value)| {
            (
                key.to_string(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();

    let new_token = headers.get("new-token").cloned();
    let header_success = headers
        .get("success")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let raw_body = response.text()?;
    let body = serde_json::from_str::<serde_json::Value>(&raw_body)
        .unwrap_or_else(|_| serde_json::Value::String(raw_body.clone()));

    let code = body.get("code").and_then(|v| v.as_i64());
    let msg = body
        .get("msg")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let should_reload = body
        .get("data")
        .and_then(|v| v.get("reload"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(ApiResponse {
        status: status.as_u16(),
        headers,
        body,
        raw_body,
        code,
        msg,
        success: code == Some(0) || header_success,
        new_token,
        unauthorized: status.as_u16() == 401,
        should_reload,
    })
}
