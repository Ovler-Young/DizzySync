import { Alert, Card, Descriptions, Tag, Typography } from "antd";
import type { ReactNode } from "react";
import { useI18n } from "../i18n.tsx";
import type { StatusResponse } from "../types.ts";

interface StatusCardProps {
  status: StatusResponse | null;
}

export function StatusCard({ status }: StatusCardProps) {
  const { t } = useI18n();

  if (!status) {
    return <Alert showIcon={true} type="info" message={t("status.loading")} />;
  }

  let job = t("status.idle");
  let jobColor = "default";
  if (status.job.state === "running") {
    job = status.job.kind;
    jobColor = "processing";
  }

  let loginColor = "orange";
  let loginText = t("status.notReady");
  if (status.ready) {
    loginColor = "green";
    loginText = t("status.ready");
  }

  let userText = "-";
  if (status.user) {
    userText = `${status.user.username} (${status.user.uid})`;
  }

  let scheduleColor = "default";
  let scheduleText = t("status.scheduleDisabled");
  let scheduleCron = "-";
  if (status.schedule.enabled) {
    scheduleColor = "blue";
    scheduleText = t("status.scheduleEnabled");
    scheduleCron = status.schedule.cron;
  }

  const formatTime = (timestamp: number | null) => {
    if (!timestamp) {
      return "-";
    }
    return new Date(timestamp * 1000).toLocaleString();
  };

  let scheduleError: ReactNode = null;
  if (status.schedule.last_error) {
    scheduleError = (
      <Typography.Paragraph style={{ marginBottom: 0, marginTop: 16 }} type="danger">
        {t("status.scheduleLastError", { message: status.schedule.last_error })}
      </Typography.Paragraph>
    );
  }

  let lastError: ReactNode = null;
  if (status.last_error) {
    lastError = (
      <Typography.Paragraph style={{ marginBottom: 0, marginTop: 16 }} type="danger">
        {t("status.lastError", { message: status.last_error })}
      </Typography.Paragraph>
    );
  }

  return (
    <Card title={t("status.title")}>
      <Descriptions bordered={true} column={{ xs: 1, sm: 2, lg: 4 }} size="small">
        <Descriptions.Item label={t("status.api")}>
          <Tag color="green">{status.status}</Tag>
        </Descriptions.Item>
        <Descriptions.Item label={t("status.login")}>
          <Tag color={loginColor}>{loginText}</Tag>
        </Descriptions.Item>
        <Descriptions.Item label={t("status.user")}>{userText}</Descriptions.Item>
        <Descriptions.Item label={t("status.syncJob")}>
          <Tag color={jobColor}>{job}</Tag>
        </Descriptions.Item>
        <Descriptions.Item label={t("status.schedule")}>
          <Tag color={scheduleColor}>{scheduleText}</Tag>
        </Descriptions.Item>
        <Descriptions.Item label={t("status.scheduleCron")}>{scheduleCron}</Descriptions.Item>
        <Descriptions.Item label={t("status.nextRun")}>
          {formatTime(status.schedule.next_run)}
        </Descriptions.Item>
        <Descriptions.Item label={t("status.lastRun")}>
          {formatTime(status.schedule.last_run)}
        </Descriptions.Item>
      </Descriptions>
      {scheduleError}
      {lastError}
    </Card>
  );
}
