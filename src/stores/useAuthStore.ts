import { create } from "zustand";
import { persist } from "zustand/middleware";

type UserInfo = {
  ID?: string;
  [key: string]: unknown;
};

type AuthState = {
  token: string;
  userInfo: UserInfo | null;
  setToken: (token: string) => void;
  setUserInfo: (userInfo: UserInfo | null) => void;
  clearAuth: () => void;
};

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      token: "",
      userInfo: null,
      setToken: (token) => set({ token }),
      setUserInfo: (userInfo) => set({ userInfo }),
      clearAuth: () => set({ token: "", userInfo: null })
    }),
    {
      name: "gvac-auth"
    }
  )
);
