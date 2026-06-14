import { ReloadOutlined } from "@ant-design/icons";
import { Alert, Button, Card, Empty, List, Space, Tag, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { api } from "../api.ts";
import { useI18n } from "../i18n.tsx";
import type { LogEntry } from "../types.ts";

const { Text } = Typography;

const levelColor: Record<LogEntry["level"], string> = {
  error: "red",
  warn: "orange",
  info: "blue",
};

function formatTime(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

export function LogViewer() {
  const { t } = useI18n();
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadLogs = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setLogs(await api.logs());
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadLogs();
  }, [loadLogs]);

  return (
    <Card
      title={t("logs.title")}
      extra={
        <Button icon={<ReloadOutlined />} loading={loading} onClick={loadLogs}>
          {t("logs.refresh")}
        </Button>
      }
    >
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <Alert showIcon={true} type="info" message={t("logs.description")} />
        {error ? <Alert showIcon={true} type="error" message={error} /> : null}
        {logs.length === 0 ? (
          <Empty description={t("logs.empty")} />
        ) : (
          <List
            dataSource={logs}
            loading={loading}
            renderItem={(item) => (
              <List.Item>
                <Space align="start" size="middle">
                  <Tag color={levelColor[item.level]}>{item.level.toUpperCase()}</Tag>
                  <div>
                    <Text type="secondary">{formatTime(item.timestamp)}</Text>
                    <div>{item.message}</div>
                  </div>
                </Space>
              </List.Item>
            )}
          />
        )}
      </Space>
    </Card>
  );
}
