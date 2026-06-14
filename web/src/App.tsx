import { LoginOutlined, TranslationOutlined } from "@ant-design/icons";
import {
  Alert,
  App as AntApp,
  Button,
  Card,
  Dropdown,
  Form,
  Input,
  Layout,
  Space,
  Tabs,
  Typography,
} from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { ApiError, api, apiKeyStorageKey } from "./api.ts";
import { AlbumDetailDrawer } from "./components/AlbumDetailDrawer.tsx";
import { AlbumTable } from "./components/AlbumTable.tsx";
import { ConfigForm } from "./components/ConfigForm.tsx";
import { ConfigGuide, type ConfigGuideSection } from "./components/ConfigGuide.tsx";
import { LogViewer } from "./components/LogViewer.tsx";
import { StatusCard } from "./components/StatusCard.tsx";
import { SyncControls } from "./components/SyncControls.tsx";
import { type Language, useI18n } from "./i18n.tsx";
import type { ConfigResponse, DiscInfo, DiscListItem, StatusResponse } from "./types.ts";

const { Footer, Header } = Layout;
const { Title, Text } = Typography;

const languageOptions: Array<{ label: string; value: Language }> = [
  { label: "中文", value: "zh-CN" },
  { label: "English", value: "en-US" },
];

interface LoginValues {
  apiKey: string;
}

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
  const [activeGuideKey, setActiveGuideKey] = useState<ConfigGuideSection>("user");

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

  const login = useCallback(
    async (values: LoginValues) => {
      saveApiKey(values.apiKey);
      await refreshAll();
    },
    [refreshAll, saveApiKey],
  );

  const closeAlbumDetail = useCallback(() => {
    setDetail(null);
  }, []);

  const languageMenuItems = useMemo(
    () =>
      languageOptions.map((option) => ({
        key: option.value,
        label: option.label,
      })),
    [],
  );

  const languageButton = (className?: string) => (
    <Dropdown
      menu={{
        items: languageMenuItems,
        selectedKeys: [language],
        onClick: ({ key }) => setLanguage(key as Language),
      }}
      trigger={["click"]}
    >
      <Button aria-label={t("app.language")} className={className} icon={<TranslationOutlined />}>
        {t("app.language")}
      </Button>
    </Dropdown>
  );

  const onboarding = (
    <Space direction="vertical" size="large" style={{ width: "100%" }}>
      <Alert
        showIcon={true}
        type="info"
        message={t("onboarding.notReady")}
        description={t("onboarding.notReadyDescription")}
      />
      <div className="settings-grid">
        <ConfigForm
          config={config}
          mode="onboarding"
          onFocusGuide={setActiveGuideKey}
          onSaved={handleConfigSaved}
        />
        <ConfigGuide activeKey={activeGuideKey} onActiveKeyChange={setActiveGuideKey} />
      </div>
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
        key: "logs",
        label: t("tabs.logs"),
        children: <LogViewer />,
      },
      {
        key: "settings",
        label: t("tabs.settings"),
        children: (
          <div className="settings-grid">
            <ConfigForm
              config={config}
              onFocusGuide={setActiveGuideKey}
              onSaved={handleConfigSaved}
            />
            <ConfigGuide activeKey={activeGuideKey} onActiveKeyChange={setActiveGuideKey} />
          </div>
        ),
      },
    ],
    [
      activeGuideKey,
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

  if (authRequired) {
    return (
      <Layout className="auth-layout">
        <div className="auth-language">{languageButton()}</div>
        <Card className="login-card">
          <Space direction="vertical" size="large" style={{ width: "100%" }}>
            <div>
              <Title level={2}>{t("auth.loginTitle")}</Title>
              <Text type="secondary">{t("auth.requiredDescription")}</Text>
            </div>
            {error ? <Alert showIcon={true} type="error" message={error} /> : null}
            <Form<LoginValues> initialValues={{ apiKey }} layout="vertical" onFinish={login}>
              <Form.Item
                label={t("app.apiKey.placeholder")}
                name="apiKey"
                rules={[{ required: true, message: t("config.webPasswordRequired") }]}
              >
                <Input.Password autoFocus={true} autoComplete="current-password" />
              </Form.Item>
              <Button
                block={true}
                htmlType="submit"
                icon={<LoginOutlined />}
                loading={loading}
                type="primary"
              >
                {t("auth.login")}
              </Button>
            </Form>
          </Space>
        </Card>
      </Layout>
    );
  }

  return (
    <Layout>
      <Header className="app-header">
        <div className="app-logo">{t("app.title")}</div>
        <Space>{languageButton()}</Space>
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
      <Footer className="app-footer">
        <Space direction="vertical" size={4}>
          <Text type="secondary">{t("footer.disclaimer")}</Text>
          <Space split={<Text type="secondary">·</Text>} wrap={true}>
            <Typography.Link href="https://github.com/Ovler-Young/DizzySync" target="_blank">
              {t("footer.source")}
            </Typography.Link>
            <Typography.Link href="https://www.dizzylab.net" target="_blank">
              {t("footer.dizzylab")}
            </Typography.Link>
            <Text type="secondary">{t("footer.credit")}</Text>
          </Space>
        </Space>
      </Footer>
      <AlbumDetailDrawer album={detail} onClose={closeAlbumDetail} onSync={syncAlbum} />
    </Layout>
  );
}
