import { Alert, Card, Descriptions, Tag, Typography } from "antd";
import type { ReactNode } from "react";
import { useI18n } from "../i18n.tsx";
import type { StatusResponse, UserInfo } from "../types.ts";

interface StatusCardProps {
  status: StatusResponse | null;
}

function resolveUsers(status: StatusResponse): UserInfo[] {
  if (status.users.length > 0) {
    return status.users;
  }
  if (status.user) {
    return [status.user];
  }
  return [];
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

  const users = resolveUsers(status);
  let userText = "-";
  if (users.length > 0) {
    userText = users.map((user) => `${user.username} (${user.uid})`).join(", ");
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
      </Descriptions>
      {lastError}
    </Card>
  );
}
