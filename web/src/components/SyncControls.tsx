import { PlayCircleOutlined } from "@ant-design/icons";
import { Alert, Button, Card, Space } from "antd";

interface SyncControlsProps {
  disabled: boolean;
  onSyncAll: () => void;
}

export default function SyncControls({ disabled, onSyncAll }: SyncControlsProps) {
  return (
    <Card title="同步控制">
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <Alert
          showIcon={true}
          type="info"
          message="同一时间只允许一个同步任务运行。任务启动后可在状态区域查看运行状态。"
        />
        <Button
          disabled={disabled}
          icon={<PlayCircleOutlined />}
          type="primary"
          onClick={onSyncAll}
        >
          同步全部已购专辑
        </Button>
      </Space>
    </Card>
  );
}
