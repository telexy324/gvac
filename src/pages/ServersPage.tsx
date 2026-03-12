import { useEffect, useState } from "react";
import { toast } from "sonner";

import { getServerList } from "@/api/cmdb";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

type Server = {
  ID: number;
  hostname: string;
  manageIp: string;
  sshPort: number;
};

type SearchState = {
  hostname: string;
  manageIp: string;
};

const defaultSearch: SearchState = {
  hostname: "",
  manageIp: ""
};

type ServersPageProps = {
  refreshTick?: number;
};

export default function ServersPage({ refreshTick = 0 }: ServersPageProps) {
  const [items, setItems] = useState<Server[]>([]);
  const [search, setSearch] = useState<SearchState>(defaultSearch);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);
  const [total, setTotal] = useState(0);

  const query = async (options?: {
    nextPage?: number;
    nextPageSize?: number;
    nextSearch?: SearchState;
  }) => {
    try {
      const nextPage = options?.nextPage ?? page;
      const nextPageSize = options?.nextPageSize ?? pageSize;
      const nextSearch = options?.nextSearch ?? search;
      const res = await getServerList({
        page: nextPage,
        pageSize: nextPageSize,
        ...nextSearch
      });

      if (res?.code === 0 || res?.success) {
        setItems(res.data?.list || []);
        setTotal(res.data?.total || 0);
      }
    } catch {
      toast.error("获取服务器列表失败");
    }
  };

  useEffect(() => {
    void query();
  }, [page, pageSize, refreshTick]);

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
                setPage(1);
                void query({ nextPage: 1 });
              }}
            >
              查询
            </Button>
            <Button
              variant="outline"
              onClick={() => {
                setSearch(defaultSearch);
                setPage(1);
                void query({ nextPage: 1, nextSearch: defaultSearch });
              }}
            >
              重置
            </Button>
          </div>
        </div>

        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>ID</TableHead>
              <TableHead>服务器</TableHead>
              <TableHead>管理IP</TableHead>
              <TableHead>端口</TableHead>
              <TableHead>操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {items.map((row) => (
              <TableRow key={row.ID}>
                <TableCell>{row.ID}</TableCell>
                <TableCell>{row.hostname}</TableCell>
                <TableCell>{row.manageIp}</TableCell>
                <TableCell>{row.sshPort}</TableCell>
                <TableCell>
                  <Button size="sm" variant="secondary" onClick={() => undefined}>
                    执行
                  </Button>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>

        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <span>共 {total} 条</span>
          <div className="flex items-center gap-2">
            <select
              className="h-8 rounded-md border bg-background px-2"
              value={pageSize}
              onChange={(event) => {
                const nextPageSize = Number(event.target.value);
                setPage(1);
                setPageSize(nextPageSize);
              }}
            >
              {[10, 30, 50, 100].map((value) => (
                <option key={value} value={value}>
                  {value}/页
                </option>
              ))}
            </select>
            <Button
              variant="outline"
              size="sm"
              disabled={page <= 1}
              onClick={() => setPage((prev) => prev - 1)}
            >
              上一页
            </Button>
            <span>第 {page} 页</span>
            <Button
              variant="outline"
              size="sm"
              disabled={page * pageSize >= total}
              onClick={() => setPage((prev) => prev + 1)}
            >
              下一页
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
