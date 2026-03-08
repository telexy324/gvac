use std::collections::HashMap;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::Method;

use crate::errors::{AppError, AppResult};
use crate::models::{HttpRequest, HttpResponse};

const DEFAULT_TIMEOUT_MS: u64 = 30_000;

fn normalize_url(url: &str) -> AppResult<&str> {
    let normalized = url.trim();
    if normalized.is_empty() {
        return Err(AppError::InvalidInput("url is required".to_string()));
    }
    Ok(normalized)
}

fn build_client(timeout_ms: Option<u64>) -> AppResult<Client> {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS).max(1));
    Ok(Client::builder().timeout(timeout).build()?)
}

fn parse_method(method: &str) -> AppResult<Method> {
    Method::from_bytes(method.trim().as_bytes())
        .map_err(|_| AppError::InvalidInput(format!("invalid HTTP method: {method}")))
}

fn apply_headers(
    mut builder: reqwest::blocking::RequestBuilder,
    headers: Option<HashMap<String, String>>,
) -> reqwest::blocking::RequestBuilder {
    if let Some(headers) = headers {
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
    }
    builder
}

pub fn send(request: HttpRequest) -> AppResult<HttpResponse> {
    let url = normalize_url(&request.url)?;
    let method = parse_method(&request.method)?;
    let client = build_client(request.timeout_ms)?;

    let mut builder = client.request(method, url);

    if let Some(query) = &request.query {
        builder = builder.query(query);
    }

    builder = apply_headers(builder, request.headers);

    if let Some(body) = request.body {
        builder = builder.json(&body);
    }

    let response = builder.send()?;
    map_response(response)
}

pub fn get(
    url: &str,
    headers: Option<HashMap<String, String>>,
    query: Option<HashMap<String, String>>,
    timeout_ms: Option<u64>,
) -> AppResult<HttpResponse> {
    send(HttpRequest {
        method: "GET".to_string(),
        url: url.to_string(),
        headers,
        query,
        body: None,
        timeout_ms,
    })
}

pub fn post_json(
    url: &str,
    headers: Option<HashMap<String, String>>,
    query: Option<HashMap<String, String>>,
    body: serde_json::Value,
    timeout_ms: Option<u64>,
) -> AppResult<HttpResponse> {
    send(HttpRequest {
        method: "POST".to_string(),
        url: url.to_string(),
        headers,
        query,
        body: Some(body),
        timeout_ms,
    })
}

fn map_response(response: reqwest::blocking::Response) -> AppResult<HttpResponse> {
    let status = response.status();

    let headers = response
        .headers()
        .iter()
        .map(|(key, value)| {
            (
                key.to_string(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();

    let body = response.text()?;

    Ok(HttpResponse {
        status: status.as_u16(),
        ok: status.is_success(),
        headers,
        body,
    })
}
