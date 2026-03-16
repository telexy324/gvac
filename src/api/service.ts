import { toast } from "sonner";

import {
  createApiService,
  initApiClient,
  isTauriRuntime,
  setApiAuth,
  type ApiRequestConfig,
  type ApiResponse
} from "@/lib/api";
import { useAuthStore } from "@/stores/useAuthStore";

const DEFAULT_BASE_URL = "http://188.4.32.11:44480/api";
const DEFAULT_BROWSER_BASE_URL = "/api";
let initialized = false;

const ensureClient = async () => {
  if (initialized) {
    return;
  }

  const tauriBaseUrl = (import.meta.env.VITE_TAURI_BASE_API as string | undefined)?.trim();
  const browserBaseUrl = (import.meta.env.VITE_BASE_API as string | undefined)?.trim();
  const baseURL = isTauriRuntime()
    ? tauriBaseUrl || DEFAULT_BASE_URL
    : browserBaseUrl || DEFAULT_BROWSER_BASE_URL;
  await initApiClient(baseURL, 60_000);

  const { token, userInfo } = useAuthStore.getState();
  await setApiAuth(token || "", userInfo?.ID || "");
  initialized = true;
};

const service = createApiService({
  onTokenRefresh: (token) => {
    useAuthStore.getState().setToken(token);
  },
  onUnauthorized: () => {
    useAuthStore.getState().clearAuth();
    window.location.hash = "#/";
  },
  onReload: () => {
    useAuthStore.getState().clearAuth();
    window.location.hash = "#/";
  },
  onError: (message) => {
    toast.error(message || "网络请求失败");
  }
});

export const apiRequest = async <T = unknown>(config: ApiRequestConfig): Promise<ApiResponse<T>> => {
  await ensureClient();
  return service<T>(config);
};
