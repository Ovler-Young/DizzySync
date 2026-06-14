import { KeyOutlined, ReloadOutlined } from "@ant-design/icons";
import { Alert, App as AntApp, Button, Card, Input, Layout, Space, Tabs, Typography } from "antd";
import type { ChangeEvent } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { api, apiKeyStorageKey } from "./api.ts";
import { AlbumDetailDrawer } from "./components/AlbumDetailDrawer.tsx";
import { AlbumTable } from "./components/AlbumTable.tsx";
import { ConfigForm } from "./components/ConfigForm.tsx";
import { ConfigGuide } from "./components/ConfigGuide.tsx";
import { StatusCard } from "./components/StatusCard.tsx";
import { SyncControls } from "./components/SyncControls.tsx";
import type { ConfigResponse, DiscInfo, DiscListItem, StatusResponse } from "./types.ts";

const { Header } = Layout;
const { Title, Text } = Typography;

export function App() {
  const { message } = AntApp.useApp();
  const [apiKey, setApiKey] = useState(
    () => globalThis.localStorage.getItem(apiKeyStorageKey) ?? "",
  );
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [config, setConfig] = useState<ConfigResponse | null>(null);
  const [albums, setAlbums] = useState<DiscListItem[]>([]);
  const [detail, setDetail] = useState<DiscInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isRunning = status?.job.state === "running";

  const saveApiKey = useCallback((value: string) => {
    setApiKey(value);
    if (value.trim()) {
      globalThis.localStorage.setItem(apiKeyStorageKey, value.trim());
    } else {
      globalThis.localStorage.removeItem(apiKeyStorageKey);
    }
  }, []);

  const loadStatus = useCallback(async () => {
    const nextStatus = await api.status();
    setStatus(nextStatus);
    return nextStatus;
  }, []);

  const refreshAll = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [nextStatus, nextConfig] = await Promise.all([api.status(), api.config()]);
      setStatus(nextStatus);
      setConfig(nextConfig);
      if (nextStatus.ready) {
        setAlbums(await api.albums());
      }
    } catch (caught) {
      const text = caught instanceof Error ? caught.message : String(caught);
      setError(text);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshAll().catch((caught: unknown) => {
      setError(caught instanceof Error ? caught.message : String(caught));
    });
  }, [refreshAll]);

  useEffect(() => {
    if (!isRunning) {
      return;
    }

    const timer = globalThis.setInterval(() => {
      loadStatus().catch((caught: unknown) => {
        setError(caught instanceof Error ? caught.message : String(caught));
      });
    }, 2500);

    return () => globalThis.clearInterval(timer);
  }, [isRunning, loadStatus]);

  const showAlbum = useCallback(
    async (id: string) => {
      setLoading(true);
      try {
        setDetail(await api.album(id));
      } catch (caught) {
        message.error(caught instanceof Error ? caught.message : String(caught));
      } finally {
        setLoading(false);
      }
    },
    [message],
  );

  const syncAll = useCallback(async () => {
    try {
      const response = await api.syncAll();
      message.success(response.message);
      await loadStatus();
    } catch (caught) {
      message.error(caught instanceof Error ? caught.message : String(caught));
    }
  }, [loadStatus, message]);

  const syncAlbum = useCallback(
    async (id: string) => {
      try {
        const response = await api.syncAlbum(id);
        message.success(response.message);
        await loadStatus();
      } catch (caught) {
        message.error(caught instanceof Error ? caught.message : String(caught));
      }
    },
    [loadStatus, message],
  );

  const handleConfigSaved = useCallback(
    (nextConfig: ConfigResponse) => {
      setConfig(nextConfig);
      loadStatus().catch((caught: unknown) => {
        message.error(caught instanceof Error ? caught.message : String(caught));
      });
    },
    [loadStatus, message],
  );

  const closeAlbumDetail = useCallback(() => {
    setDetail(null);
  }, []);

  const handleApiKeyChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      saveApiKey(event.target.value);
    },
    [saveApiKey],
  );

  const tabItems = useMemo(
    () => [
      {
        key: "dashboard",
        label: "控制台",
        children: (
          <Space direction="vertical" size="large" style={{ width: "100%" }}>
            <StatusCard status={status} />
            <SyncControls disabled={!status?.ready || isRunning} onSyncAll={syncAll} />
            <AlbumTable
              albums={albums}
              loading={loading}
              onRefresh={refreshAll}
              onShow={showAlbum}
              onSync={syncAlbum}
              syncDisabled={isRunning}
            />
          </Space>
        ),
      },
      {
        key: "settings",
        label: "设置",
        children: (
          <Space direction="vertical" size="large" style={{ width: "100%" }}>
            <ConfigGuide />
            <ConfigForm config={config} onSaved={handleConfigSaved} />
          </Space>
        ),
      },
    ],
    [
      albums,
      config,
      isRunning,
      handleConfigSaved,
      loading,
      refreshAll,
      showAlbum,
      status,
      syncAlbum,
      syncAll,
    ],
  );

  return (
    <Layout>
      <Header style={{ alignItems: "center", display: "flex", justifyContent: "space-between" }}>
        <div className="app-logo">DizzySync 控制台</div>
        <Space>
          <Input.Password
            allowClear={true}
            placeholder="API Key（如已启用）"
            prefix={<KeyOutlined />}
            style={{ width: 280 }}
            value={apiKey}
            onChange={handleApiKeyChange}
          />
          <Button icon={<ReloadOutlined />} loading={loading} onClick={refreshAll}>
            刷新
          </Button>
        </Space>
      </Header>
      <main className="page-content">
        <Space direction="vertical" size="large" style={{ width: "100%" }}>
          <div>
            <Title level={2} style={{ marginBottom: 4 }}>
              音乐同步与配置管理
            </Title>
            <Text type="secondary">
              通过同一个 Rust 服务管理配置、查看专辑并触发同步。Docker 部署只暴露一个端口。
            </Text>
          </div>
          {error ? (
            <Alert showIcon={true} type="error" message="请求失败" description={error} />
          ) : null}
          <Card>
            <Tabs items={tabItems} />
          </Card>
        </Space>
      </main>
      <AlbumDetailDrawer album={detail} onClose={closeAlbumDetail} onSync={syncAlbum} />
    </Layout>
  );
}
