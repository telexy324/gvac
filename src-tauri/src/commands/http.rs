use tauri::State;

use crate::errors::AppResult;
use crate::models::{HttpGetRequest, HttpPostJsonRequest, HttpRequest, HttpResponse};
use crate::services::http;
use crate::state::AppState;

#[tauri::command]
pub fn http_request(_state: State<'_, AppState>, request: HttpRequest) -> AppResult<HttpResponse> {
    http::send(request)
}

#[tauri::command]
pub fn http_get(_state: State<'_, AppState>, request: HttpGetRequest) -> AppResult<HttpResponse> {
    http::get(
        &request.url,
        request.headers,
        request.query,
        request.timeout_ms,
    )
}

#[tauri::command]
pub fn http_post_json(
    _state: State<'_, AppState>,
    request: HttpPostJsonRequest,
) -> AppResult<HttpResponse> {
    http::post_json(
        &request.url,
        request.headers,
        request.query,
        request.body,
        request.timeout_ms,
    )
}
