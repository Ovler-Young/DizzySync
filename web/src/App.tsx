import { KeyOutlined, ReloadOutlined } from "@ant-design/icons";
import {
  Alert,
  App as AntApp,
  Button,
  Card,
  Input,
  Layout,
  Select,
  Space,
  Tabs,
  Typography,
} from "antd";
import type { ChangeEvent } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { ApiError, api, apiKeyStorageKey } from "./api.ts";
import { AlbumDetailDrawer } from "./components/AlbumDetailDrawer.tsx";
import { AlbumTable } from "./components/AlbumTable.tsx";
import { ConfigForm } from "./components/ConfigForm.tsx";
import { ConfigGuide } from "./components/ConfigGuide.tsx";
import { StatusCard } from "./components/StatusCard.tsx";
import { SyncControls } from "./components/SyncControls.tsx";
import { type Language, useI18n } from "./i18n.tsx";
import type { ConfigResponse, DiscInfo, DiscListItem, StatusResponse } from "./types.ts";

const { Header } = Layout;
const { Title, Text } = Typography;

const languageOptions: Array<{ label: string; value: Language }> = [
  { label: "中文", value: "zh-CN" },
  { label: "English", value: "en-US" },
];

export function App() {
  const { message } = AntApp.useApp();
  const { language, setLanguage, t } = useI18n();
  const [apiKey, setApiKey] = useState(
    () => globalThis.localStorage.getItem(apiKeyStorageKey) ?? "",
  );
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [config, setConfig] = useState<ConfigResponse | null>(null);
  const [albums, setAlbums] = useState<DiscListItem[]>([]);
  const [detail, setDetail] = useState<DiscInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [authRequired, setAuthRequired] = useState(false);

  const isRunning = status?.job.state === "running";
  const needsOnboarding = status && !authRequired ? !(status.configured && status.ready) : false;

  const saveApiKey = useCallback((value: string) => {
    setApiKey(value);
    if (value.trim()) {
      globalThis.localStorage.setItem(apiKeyStorageKey, value.trim());
    } else {
      globalThis.localStorage.removeItem(apiKeyStorageKey);
    }
  }, []);

  const refreshAll = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const nextStatus = await api.status();
      setStatus(nextStatus);
      try {
        const nextConfig = await api.config();
        setConfig(nextConfig);
        setAuthRequired(false);
      } catch (caught) {
        if (caught instanceof ApiError && caught.status === 401) {
          setAuthRequired(true);
          setConfig(null);
          setAlbums([]);
          setDetail(null);
          return;
        }
        throw caught;
      }
      if (nextStatus.ready) {
        setAlbums(await api.albums());
      } else {
        setAlbums([]);
        setDetail(null);
      }
    } catch (caught) {
      const text = caught instanceof Error ? caught.message : String(caught);
      setError(text);
    } finally {
      setLoading(false);
    }
  }, []);

  const loadStatus = useCallback(async () => {
    const nextStatus = await api.status();
    setStatus((previousStatus) => {
      if (previousStatus?.job.state === "running" && nextStatus.job.state === "idle") {
        refreshAll().catch((caught: unknown) => {
          setError(caught instanceof Error ? caught.message : String(caught));
        });
      }
      return nextStatus;
    });
    return nextStatus;
  }, [refreshAll]);

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
    (nextConfig: ConfigResponse, nextApiKey?: string) => {
      if (nextApiKey !== undefined) {
        saveApiKey(nextApiKey);
      }
      setConfig(nextConfig);
      refreshAll().catch((caught: unknown) => {
        message.error(caught instanceof Error ? caught.message : String(caught));
      });
    },
    [message, refreshAll, saveApiKey],
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

  const onboarding = (
    <Space direction="vertical" size="large" style={{ width: "100%" }}>
      <Alert
        showIcon={true}
        type="info"
        message={t("onboarding.notReady")}
        description={t("onboarding.notReadyDescription")}
      />
      <ConfigForm config={config} mode="onboarding" onSaved={handleConfigSaved} />
      <ConfigGuide />
    </Space>
  );

  const tabItems = useMemo(
    () => [
      ...(needsOnboarding
        ? [
            {
              key: "onboarding",
              label: t("tabs.onboarding"),
              children: onboarding,
            },
          ]
        : []),
      {
        key: "dashboard",
        label: t("tabs.dashboard"),
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
              syncDisabled={Boolean(isRunning)}
            />
          </Space>
        ),
      },
      {
        key: "settings",
        label: t("tabs.settings"),
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
      handleConfigSaved,
      isRunning,
      loading,
      needsOnboarding,
      onboarding,
      refreshAll,
      showAlbum,
      status,
      syncAlbum,
      syncAll,
      t,
    ],
  );

  return (
    <Layout>
      <Header style={{ alignItems: "center", display: "flex", justifyContent: "space-between" }}>
        <div className="app-logo">{t("app.title")}</div>
        <Space>
          <Select
            aria-label={t("app.language")}
            options={languageOptions}
            style={{ width: 120 }}
            value={language}
            onChange={setLanguage}
          />
          <Input.Password
            allowClear={true}
            placeholder={t("app.apiKey.placeholder")}
            prefix={<KeyOutlined />}
            style={{ width: 280 }}
            value={apiKey}
            onChange={handleApiKeyChange}
          />
          <Button icon={<ReloadOutlined />} loading={loading} onClick={refreshAll}>
            {t("app.refresh")}
          </Button>
        </Space>
      </Header>
      <main className="page-content">
        <Space direction="vertical" size="large" style={{ width: "100%" }}>
          <div>
            <Title level={2} style={{ marginBottom: 4 }}>
              {needsOnboarding ? t("onboarding.title") : t("app.heading")}
            </Title>
            <Text type="secondary">
              {needsOnboarding ? t("onboarding.welcome") : t("app.subtitle")}
            </Text>
          </div>
          {authRequired ? (
            <Alert
              showIcon={true}
              type="warning"
              message={t("auth.required")}
              description={t("auth.requiredDescription")}
            />
          ) : null}
          {error ? (
            <Alert
              showIcon={true}
              type="error"
              message={t("app.error.title")}
              description={error}
            />
          ) : null}
          <Card>
            <Tabs
              defaultActiveKey={needsOnboarding ? "onboarding" : "dashboard"}
              items={tabItems}
            />
          </Card>
        </Space>
      </main>
      <AlbumDetailDrawer album={detail} onClose={closeAlbumDetail} onSync={syncAlbum} />
    </Layout>
  );
}
