import { type ApiResponse } from "@/lib/api";
import { apiRequest } from "@/api/service";

export type Server = {
  ID: number;
  hostname: string;
  displayName?: string;
  architecture?: string;
  manageIp: string;
  sshPort: number;
  os?: string;
  osVersion?: string;
  systemId?: number;
};

export type ServerListData = {
  list: Server[];
  total: number;
};

export const getServerList = async (data: Record<string, unknown>): Promise<ApiResponse<ServerListData>> => {
  return apiRequest<ServerListData>({
    url: "/cmdb/getServerList",
    method: "POST",
    data
  });
};

export type System = {
  ID: number;
  name: string;
};

export type SystemListData = {
  systems: System[];
  total: number;
};

export const getSystemList = async (data: Record<string, unknown>): Promise<ApiResponse<SystemListData>> => {
  return apiRequest<SystemListData>({
    url: "/cmdb/getSystemList",
    method: "POST",
    data
  });
};

export const getAdminSystems = async (): Promise<ApiResponse<SystemListData>> => {
  return apiRequest<SystemListData>({
    url: "/cmdb/getAdminSystems",
    method: "POST",
  });
};