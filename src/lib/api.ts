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

type BrowserApiState = {
  baseURL: string;
  timeout: number;
  token?: string;
  userId?: string;
};

const browserApiState: BrowserApiState = {
  baseURL: "",
  timeout: 60000
};

export const isTauriRuntime = () => {
  if (typeof window === "undefined") {
    return false;
  }
  return "__TAURI_INTERNALS__" in window;
};

const resolveAbsoluteUrl = (baseURL: string, url: string) => {
  if (/^https?:\/\//i.test(url)) {
    return url;
  }
  const base = baseURL.trim().replace(/\/+$/, "");
  const path = url.trim();
  if (!base) {
    throw new Error("base_url is not set; provide an absolute url or call api_set_base_url first");
  }
  if (base.startsWith("/")) {
    const origin = typeof window !== "undefined" ? window.location.origin : "http://127.0.0.1:1420";
    const basePath = base.startsWith("/") ? base : `/${base}`;
    if (path.startsWith("/")) {
      return `${origin}${basePath}${path}`;
    }
    return `${origin}${basePath}/${path}`;
  }
  if (path.startsWith("/")) {
    return `${base}${path}`;
  }
  return `${base}/${path}`;
};

const parseResponseBody = (rawBody: string): unknown => {
  if (!rawBody) {
    return "";
  }
  try {
    return JSON.parse(rawBody);
  } catch {
    return rawBody;
  }
};

const fetchApiRequest = async <T = unknown>(config: ApiRequestConfig): Promise<ApiResponse<T>> => {
  const timeout = Math.max(1, config.timeout ?? browserApiState.timeout ?? 60000);
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeout);

  try {
    const url = new URL(resolveAbsoluteUrl(browserApiState.baseURL, config.url));
    if (config.params) {
      const search = new URLSearchParams(config.params);
      search.forEach((value, key) => {
        url.searchParams.set(key, value);
      });
    }

    const headers = new Headers(config.headers);
    headers.set("Content-Type", "application/json");

    if (config.useAuth ?? true) {
      headers.set("x-token", browserApiState.token ?? "");
      headers.set("x-user-id", browserApiState.userId ?? "");
    }

    const response = await fetch(url.toString(), {
      method: (config.method ?? "GET").toUpperCase(),
      headers,
      body: config.data == null ? undefined : typeof config.data === "string" ? config.data : JSON.stringify(config.data),
      signal: controller.signal
    });

    const rawBody = await response.text();
    const body = parseResponseBody(rawBody);
    const code =
      typeof body === "object" && body && "code" in body && typeof (body as { code?: unknown }).code === "number"
        ? ((body as { code: number }).code as number)
        : undefined;
    const msg =
      typeof body === "object" && body && "msg" in body && typeof (body as { msg?: unknown }).msg === "string"
        ? ((body as { msg: string }).msg as string)
        : undefined;
    const data =
      typeof body === "object" && body && "data" in body ? ((body as { data?: T }).data as T | undefined) : undefined;
    const shouldReload =
      typeof data === "object" && data && "reload" in (data as Record<string, unknown>)
        ? Boolean((data as Record<string, unknown>).reload)
        : false;

    const responseHeaders: Record<string, string> = {};
    response.headers.forEach((value, key) => {
      responseHeaders[key] = value;
    });

    const newToken = response.headers.get("new-token") ?? undefined;
    const headerSuccess = response.headers.get("success")?.toLowerCase() === "true";

    if (newToken) {
      browserApiState.token = newToken;
    }

    return {
      status: response.status,
      headers: responseHeaders,
      body,
      data,
      rawBody,
      code,
      msg,
      success: code === 0 || headerSuccess,
      newToken,
      unauthorized: response.status === 401,
      shouldReload
    };
  } finally {
    clearTimeout(timer);
  }
};

const requestApi = async <T = unknown>(config: ApiRequestConfig): Promise<ApiResponse<T>> => {
  if (isTauriRuntime()) {
    return invoke<ApiResponse<T>>("api_request", {
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
  }
  return fetchApiRequest<T>(config);
};

export const initApiClient = async (baseURL: string, timeout = 60000) => {
  if (isTauriRuntime()) {
    await invoke("api_set_base_url", {
      request: {
        baseUrl: baseURL,
        timeoutMs: timeout
      }
    });
    return;
  }

  browserApiState.baseURL = baseURL;
  browserApiState.timeout = timeout;
};

export const setApiAuth = async (token?: string, userId?: string) => {
  if (isTauriRuntime()) {
    await invoke("api_set_auth", {
      request: {
        token,
        userId
      }
    });
    return;
  }

  browserApiState.token = token;
  browserApiState.userId = userId;
};

export const clearApiAuth = async () => {
  if (isTauriRuntime()) {
    await invoke("api_clear_auth");
    return;
  }
  browserApiState.token = undefined;
  browserApiState.userId = undefined;
};

export const createApiService = (hooks?: ApiHooks) => {
  return async <T = unknown>(config: ApiRequestConfig): Promise<ApiResponse<T>> => {
    const response = await requestApi<T>(config);

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
  if (isTauriRuntime()) {
    return invoke<ApiResponse<T>>("api_login", { data });
  }
  return requestApi<T>({
    url: "/base/login",
    method: "POST",
    data,
    useAuth: false
  });
};
