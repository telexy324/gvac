import { useEffect, useState } from "react";
import { toast } from "sonner";

import { getAdminSystems, getServerList, Server, System } from "@/api/cmdb";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";

type ServersPageProps = {
  refreshTick?: number;
};

type SearchState = {
  hostname: string;
  manageIp: string;
};

const defaultSearch: SearchState = {
  hostname: "",
  manageIp: ""
};

export default function ServersPage({ refreshTick = 0 }: ServersPageProps) {
  const [systems, setSystems] = useState<System[]>([]);
  const [expanded, setExpanded] = useState<Record<number, boolean>>({});
  const [loadingSystems, setLoadingSystems] = useState(false);
  const [loadingServers, setLoadingServers] = useState<Record<number, boolean>>({});
  const [serversBySystem, setServersBySystem] = useState<Record<number, Server[]>>({});
  const [search, setSearch] = useState<SearchState>(defaultSearch);

  const loadSystems = async () => {
    setLoadingSystems(true);
    try {
      const res = await getAdminSystems();
      if (res?.code === 0 || res?.success) {
        setSystems(res.data?.systems || []);
      } else {
        setSystems([]);
      }
    } catch {
      toast.error("获取系统列表失败");
      setSystems([]);
    } finally {
      setLoadingSystems(false);
    }
  };

  const loadServersBySystem = async (systemId: number, criteria: SearchState = search) => {
    setLoadingServers((prev) => ({ ...prev, [systemId]: true }));
    try {
      const pageSize = 100;
      let page = 1;
      let allRows: Server[] = [];
      let total = 0;

      while (true) {
        const res = await getServerList({
          page,
          pageSize,
          systemId,
          hostname: criteria.hostname,
          manageIp: criteria.manageIp
        });
        if (!(res?.code === 0 || res?.success)) {
          break;
        }
        const rows = res.data?.list || [];
        total = Number(res.data?.total || 0);
        allRows = allRows.concat(rows);
        if (rows.length < pageSize || allRows.length >= total) {
          break;
        }
        page += 1;
      }

      const filtered = allRows.filter((server) => Number(server.systemId) === systemId);
      setServersBySystem((prev) => ({ ...prev, [systemId]: filtered }));
    } catch {
      toast.error("获取服务器列表失败");
      setServersBySystem((prev) => ({ ...prev, [systemId]: [] }));
    } finally {
      setLoadingServers((prev) => ({ ...prev, [systemId]: false }));
    }
  };

  const toggleSystem = async (system: System) => {
    const nextExpanded = !expanded[system.ID];
    setExpanded((prev) => ({ ...prev, [system.ID]: nextExpanded }));
    if (nextExpanded && !serversBySystem[system.ID]) {
      await loadServersBySystem(system.ID);
    }
  };

  const runSearch = async (criteria: SearchState) => {
    setServersBySystem({});
    const expandedIds = systems
      .map((system) => system.ID)
      .filter((id) => expanded[id]);
    await Promise.all(expandedIds.map((id) => loadServersBySystem(id, criteria)));
  };

  useEffect(() => {
    setExpanded({});
    setServersBySystem({});
    void loadSystems();
  }, [refreshTick]);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Servers</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-2 md:grid-cols-3">
          <Input
            placeholder="服务器名"
            value={search.hostname}
            onChange={(event) =>
              setSearch((prev) => ({ ...prev, hostname: event.target.value }))
            }
          />
          <Input
            placeholder="管理IP"
            value={search.manageIp}
            onChange={(event) =>
              setSearch((prev) => ({ ...prev, manageIp: event.target.value }))
            }
          />
          <div className="flex gap-2">
            <Button
              onClick={() => {
                void runSearch(search);
              }}
            >
              查询
            </Button>
            <Button
              variant="outline"
              onClick={() => {
                setSearch(defaultSearch);
                void runSearch(defaultSearch);
              }}
            >
              重置
            </Button>
          </div>
        </div>
        <div className="rounded-md border border-border/60 bg-muted/20 p-3">
          {loadingSystems ? (
            <div className="text-sm text-muted-foreground">加载系统中...</div>
          ) : systems.length === 0 ? (
            <div className="text-sm text-muted-foreground">暂无系统</div>
          ) : (
            <div className="space-y-2">
              {systems.map((system) => {
                const isExpanded = !!expanded[system.ID];
                const isLoading = !!loadingServers[system.ID];
                const rows = serversBySystem[system.ID] || [];
                return (
                  <div key={system.ID} className="space-y-1">
                    <button
                      type="button"
                      className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm transition hover:bg-muted/60"
                      onClick={() => {
                        void toggleSystem(system);
                      }}
                    >
                      <span className="w-4 text-center text-muted-foreground">
                        {isExpanded ? "▾" : "▸"}
                      </span>
                      <span className="font-medium text-foreground">{system.name}</span>
                    </button>
                    {isExpanded ? (
                      <div className="space-y-1 pl-7">
                        {isLoading ? (
                          <div className="text-xs text-muted-foreground">加载服务器中...</div>
                        ) : rows.length === 0 ? (
                          <div className="text-xs text-muted-foreground">暂无服务器</div>
                        ) : (
                          rows.map((server) => (
                            <div
                              key={server.ID}
                              className="rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-muted/40"
                            >
                              <span className="font-medium text-foreground">{server.hostname}</span>
                              <span className="ml-2">({server.manageIp})</span>
                            </div>
                          ))
                        )}
                      </div>
                    ) : null}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
