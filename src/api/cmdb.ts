import { type ApiResponse } from "@/lib/api";
import { apiRequest } from "@/api/service";

type Server = {
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

type ServerListData = {
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
