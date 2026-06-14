import { Alert, Card, Descriptions, Tag, Typography } from "antd";
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

  const job = status.job.state === "running" ? `${status.job.kind}` : t("status.idle");

  return (
    <Card title={t("status.title")}>
      <Descriptions bordered={true} column={{ xs: 1, sm: 2, lg: 4 }} size="small">
        <Descriptions.Item label={t("status.api")}>
          <Tag color="green">{status.status}</Tag>
        </Descriptions.Item>
        <Descriptions.Item label={t("status.login")}>
          <Tag color={status.ready ? "green" : "orange"}>
            {status.ready ? t("status.ready") : t("status.notReady")}
          </Tag>
        </Descriptions.Item>
        <Descriptions.Item label={t("status.user")}>
          {status.user ? `${status.user.username} (${status.user.uid})` : "-"}
        </Descriptions.Item>
        <Descriptions.Item label={t("status.syncJob")}>
          <Tag color={status.job.state === "running" ? "processing" : "default"}>{job}</Tag>
        </Descriptions.Item>
      </Descriptions>
      {status.last_error ? (
        <Typography.Paragraph style={{ marginBottom: 0, marginTop: 16 }} type="danger">
          {t("status.lastError", { message: status.last_error })}
        </Typography.Paragraph>
      ) : null}
    </Card>
  );
}
