import { toast } from "sonner";

import { createApiService, initApiClient, setApiAuth, type ApiResponse } from "@/lib/api";
import { useAuthStore } from "@/stores/useAuthStore";

type LoginPayload = {
  username: string;
  password: string;
  captcha: string;
  captchaId: string;
};

type CaptchaData = {
  captchaId: string;
  picPath: string;
};

let initialized = false;
const DEFAULT_BASE_URL = "http://127.0.0.1:8888";

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

export const login = async (data: LoginPayload): Promise<ApiResponse<{ token: string; user: Record<string, unknown> }>> => {
  await ensureClient();
  const res = await service<{ token: string; user: Record<string, unknown> }>({
    url: "/base/login",
    method: "POST",
    data,
    useAuth: false
  });

  if (res.status === 401) {
    useAuthStore.getState().clearAuth();
  }

  return res;
};

export const captcha = async (data: Record<string, unknown>): Promise<ApiResponse<CaptchaData>> => {
  await ensureClient();
  return service<CaptchaData>({
    url: "/base/captcha",
    method: "POST",
    data,
    useAuth: false
  });
};
