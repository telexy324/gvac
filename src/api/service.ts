import { toast } from "sonner";

import { createApiService, initApiClient, setApiAuth, type ApiRequestConfig, type ApiResponse } from "@/lib/api";
import { useAuthStore } from "@/stores/useAuthStore";

const DEFAULT_BASE_URL = "http://127.0.0.1:8888";
let initialized = false;

const ensureClient = async () => {
  if (initialized) {
    return;
  }

  const baseURL = (import.meta.env.VITE_BASE_API as string | undefined) || DEFAULT_BASE_URL;
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

