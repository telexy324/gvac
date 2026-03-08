import { invoke } from "@tauri-apps/api/core";

export type ApiResponse<T = unknown> = {
  code?: number;
  data?: T;
  msg?: string;
  success: boolean;
  status: number;
  headers: Record<string, string>;
  newToken?: string;
  unauthorized: boolean;
  shouldReload: boolean;
  body: unknown;
  rawBody: string;
};

export type ApiRequestConfig = {
  url: string;
  method?: string;
  data?: unknown;
  params?: Record<string, string>;
  headers?: Record<string, string>;
  timeout?: number;
  useAuth?: boolean;
};

export type ApiHooks = {
  onTokenRefresh?: (token: string) => void;
  onUnauthorized?: () => void;
  onReload?: () => void;
  onError?: (message: string) => void;
};

export const initApiClient = async (baseURL: string, timeout = 60000) => {
  await invoke("api_set_base_url", {
    request: {
      baseUrl: baseURL,
      timeoutMs: timeout
    }
  });
};

export const setApiAuth = async (token?: string, userId?: string) => {
  await invoke("api_set_auth", {
    request: {
      token,
      userId
    }
  });
};

export const clearApiAuth = async () => {
  await invoke("api_clear_auth");
};

export const createApiService = (hooks?: ApiHooks) => {
  return async <T = unknown>(config: ApiRequestConfig): Promise<ApiResponse<T>> => {
    const response = await invoke<ApiResponse<T>>("api_request", {
      request: {
        url: config.url,
        method: (config.method ?? "GET").toUpperCase(),
        query: config.params,
        headers: config.headers,
        data: config.data,
        timeoutMs: config.timeout,
        useAuth: config.useAuth ?? true
      }
    });

    if (response.newToken) {
      hooks?.onTokenRefresh?.(response.newToken);
    }

    if (!response.success && response.msg) {
      hooks?.onError?.(response.msg);
    }

    if (response.shouldReload) {
      hooks?.onReload?.();
    }

    if (response.unauthorized) {
      hooks?.onUnauthorized?.();
    }

    return response;
  };
};

export const apiLogin = async <T = unknown>(data: unknown): Promise<ApiResponse<T>> => {
  return invoke<ApiResponse<T>>("api_login", { data });
};
