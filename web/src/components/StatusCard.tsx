import { Alert, Card, Descriptions, Tag, Typography } from "antd";
import type { StatusResponse } from "../types.ts";

interface StatusCardProps {
  status: StatusResponse | null;
}

export default function StatusCard({ status }: StatusCardProps) {
  if (!status) {
    return <Alert showIcon={true} type="info" message="正在读取服务状态..." />;
  }

  const job = status.job.state === "running" ? `${status.job.kind}` : "空闲";

  return (
    <Card title="服务状态">
      <Descriptions bordered={true} column={{ xs: 1, sm: 2, lg: 4 }} size="small">
        <Descriptions.Item label="API 状态">
          <Tag color="green">{status.status}</Tag>
        </Descriptions.Item>
        <Descriptions.Item label="登录状态">
          <Tag color={status.ready ? "green" : "orange"}>{status.ready ? "已就绪" : "未就绪"}</Tag>
        </Descriptions.Item>
        <Descriptions.Item label="用户">
          {status.user ? `${status.user.username} (${status.user.uid})` : "-"}
        </Descriptions.Item>
        <Descriptions.Item label="同步任务">
          <Tag color={status.job.state === "running" ? "processing" : "default"}>{job}</Tag>
        </Descriptions.Item>
      </Descriptions>
      {status.last_error ? (
        <Typography.Paragraph style={{ marginBottom: 0, marginTop: 16 }} type="danger">
          最近错误：{status.last_error}
        </Typography.Paragraph>
      ) : null}
    </Card>
  );
}
