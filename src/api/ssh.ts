import { type ApiResponse } from "@/lib/api";
import { apiRequest } from "@/api/service";
import { invoke } from "@tauri-apps/api/core";
import { SessionInfo } from "@/types";
import {Server} from "@/api/cmdb";

export type GetJumpServerTokenPayload = {
  servers: Array<Server>;
  client: string;
};

type GetJumpServerTokenData = {
  url?: string;
};

const DEFAULT_HMAC_KEY = "bastion-super-secret-key";

export type SessionPayload = {
  bh: string;
  bp: number;
  c: string;
  s: string;
  iat: number;
  exp: number;
};

const extractToken = (value?: string): string => {
  if (!value) {
    return "";
  }
  return value.replace(/^myjump:\/\//, "");
};

const encodeBase64Url = (bytes: Uint8Array): string => {
  const bin = Array.from(bytes, (byte) => String.fromCharCode(byte)).join("");
  const base64 = btoa(bin);
  return base64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
};

const decodeBase64Url = (value: string): Uint8Array => {
  const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
  const paddingLength = (4 - (normalized.length % 4)) % 4;
  const padded = normalized + "=".repeat(paddingLength);
  const bin = atob(padded);
  return Uint8Array.from(bin, (char) => char.charCodeAt(0));
};

const signHmacSha256Base64Url = async (message: string, key: string): Promise<string> => {
  const encoder = new TextEncoder();
  const cryptoKey = await crypto.subtle.importKey(
    "raw",
    encoder.encode(key),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );
  const rawSig = await crypto.subtle.sign("HMAC", cryptoKey, encoder.encode(message));
  return encodeBase64Url(new Uint8Array(rawSig));
};

export const parseSessionToken = async (
  token: string,
  hmacKey = DEFAULT_HMAC_KEY
): Promise<SessionPayload> => {
  const pureToken = extractToken(token);
  const parts = pureToken.split(".");
  if (parts.length !== 2) {
    throw new Error("invalid session format");
  }

  const payloadB64 = parts[0];
  const sigB64 = parts[1];

  // Validate base64-url format first; keep behavior close to backend parse failures.
  decodeBase64Url(sigB64);

  const expectedSigB64 = await signHmacSha256Base64Url(payloadB64, hmacKey);
  if (expectedSigB64 !== sigB64) {
    throw new Error("invalid signature");
  }

  const payloadRaw = decodeBase64Url(payloadB64);
  let payload: SessionPayload;
  try {
    const payloads = JSON.parse(new TextDecoder().decode(payloadRaw)) as SessionPayload[];
    payload = payloads[0]
  } catch {
    throw new Error("invalid payload");
  }

  if (Math.floor(Date.now() / 1000) > Number(payload.exp)) {
    throw new Error("session expired");
  }

  return payload;
};

export const getJumpServerTokenResponse = async (
  data: GetJumpServerTokenPayload
): Promise<ApiResponse<GetJumpServerTokenData>> => {
  return apiRequest<GetJumpServerTokenData>({
    url: "/jumpServer/getToken",
    method: "POST",
    data
  });
};

export const getJumpServerToken = async (data: GetJumpServerTokenPayload): Promise<string> => {
  const res = await getJumpServerTokenResponse(data);
  const urlFromBody = res.data?.url;
  const urlFromHeader = res.headers["url"] || res.headers["x-url"] || res.headers["x-token"];
  const token = extractToken(urlFromBody || urlFromHeader);

  if (!(res?.code === 0 || res?.success) || !token) {
    throw new Error(res.msg || "获取跳板机 token 失败");
  }

  return token;
};

export const getParsedJumpServerSession = async (
  data: GetJumpServerTokenPayload,
  hmacKey = DEFAULT_HMAC_KEY
): Promise<SessionPayload> => {
  const token = await getJumpServerToken(data);
  return parseSessionToken(token, hmacKey);
};

export type CreateSessionAuth =
  | { kind: "none" }
  | { kind: "password"; password: string }
  | { kind: "privateKey"; privateKeyPath: string; passphrase?: string };

export type CreateJumpSessionOptions = {
  auth?: CreateSessionAuth;
  label?: string;
  hmacKey?: string;
};

export const createTauriSessionFromJumpToken = async (
  data: GetJumpServerTokenPayload,
  options: CreateJumpSessionOptions
): Promise<SessionInfo> => {
  const payload = await getParsedJumpServerSession(data, options.hmacKey ?? DEFAULT_HMAC_KEY);
  if (!payload.bh || !payload.s || !payload.bp) {
    throw new Error("token payload incomplete for ssh session");
  }

  return invoke<SessionInfo>("create_session", {
    request: {
      label: options.label,
      host: payload.bh,
      port: payload.bp,
      username: payload.s,
      auth: options.auth ?? { kind: "none" }
    }
  });
};
