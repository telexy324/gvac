use tauri::State;

use crate::errors::AppResult;
use crate::models::{ApiAuthRequest, ApiClientConfigRequest, ApiRequest, ApiResponse};
use crate::services::api;
use crate::state::AppState;

#[tauri::command]
pub fn api_set_base_url(
    state: State<'_, AppState>,
    request: ApiClientConfigRequest,
) -> AppResult<()> {
    api::set_base_url(state, request.base_url, request.timeout_ms)
}

#[tauri::command]
pub fn api_set_auth(state: State<'_, AppState>, request: ApiAuthRequest) -> AppResult<()> {
    api::set_auth(state, request.token, request.user_id)
}

#[tauri::command]
pub fn api_clear_auth(state: State<'_, AppState>) -> AppResult<()> {
    api::clear_auth(state)
}

#[tauri::command]
pub fn api_request(state: State<'_, AppState>, request: ApiRequest) -> AppResult<ApiResponse> {
    api::send_with_middleware(state, request)
}

#[tauri::command]
pub fn api_login(state: State<'_, AppState>, data: serde_json::Value) -> AppResult<ApiResponse> {
    api::send_with_middleware(
        state,
        ApiRequest {
            url: "/base/login".to_string(),
            method: "POST".to_string(),
            headers: None,
            query: None,
            data: Some(data),
            timeout_ms: None,
            use_auth: Some(false),
        },
    )
}
