import { type ApiResponse } from "@/lib/api";
import { apiRequest } from "@/api/service";
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

export const login = async (data: LoginPayload): Promise<ApiResponse<{ token: string; user: Record<string, unknown> }>> => {
  const res = await apiRequest<{ token: string; user: Record<string, unknown> }>({
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
  return apiRequest<CaptchaData>({
    url: "/base/captcha",
    method: "POST",
    data,
    useAuth: false
  });
};
