import { ReloadOutlined } from "@ant-design/icons";
import { Alert, Button, Card, DatePicker, Empty, List, Select, Space, Tag, Typography } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "../api.ts";
import { useI18n } from "../i18n.tsx";
import type { LogEntry } from "../types.ts";

const { Text } = Typography;
const { RangePicker } = DatePicker;

interface PickerValue {
  format: (template: string) => string;
  toISOString: () => string;
}

const levelColor: Record<LogEntry["level"], string> = {
  error: "red",
  warn: "orange",
  info: "blue",
  debug: "purple",
  trace: "default",
};

function formatTime(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

export function LogViewer() {
  const { t } = useI18n();
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [date, setDate] = useState<PickerValue | null>(null);
  const [level, setLevel] = useState<string>();
  const [range, setRange] = useState<[PickerValue | null, PickerValue | null] | null>(null);

  const filters = useMemo(
    () => ({
      date: date?.format("YYYY-MM-DD"),
      level,
      start: range?.[0]?.toISOString(),
      end: range?.[1]?.toISOString(),
    }),
    [date, level, range],
  );

  const loadLogs = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setLogs(await api.logs(filters));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLoading(false);
    }
  }, [filters]);

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
        <Space wrap={true}>
          <DatePicker allowClear={true} onChange={setDate} placeholder={t("logs.filterDate")} />
          <Select
            allowClear={true}
            onChange={setLevel}
            options={[
              { label: "ERROR", value: "error" },
              { label: "WARN", value: "warn" },
              { label: "INFO", value: "info" },
              { label: "DEBUG", value: "debug" },
              { label: "TRACE", value: "trace" },
            ]}
            placeholder={t("logs.filterLevel")}
            style={{ minWidth: 140 }}
            value={level}
          />
          <RangePicker
            allowClear={true}
            onChange={(value) => setRange(value)}
            placeholder={[t("logs.filterStart"), t("logs.filterEnd")]}
            showTime={true}
          />
        </Space>
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
