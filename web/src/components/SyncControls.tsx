import { PlayCircleOutlined } from "@ant-design/icons";
import { Alert, Button, Card, Space } from "antd";

interface SyncControlsProps {
  disabled: boolean;
  onSyncAll: () => void;
}

const syncControlsTitle = "同步控制";
const syncInfoMessage = "同一时间只允许一个同步任务运行。任务启动后可在状态区域查看运行状态。";
const syncAllButtonLabel = "同步全部已购专辑";

export function SyncControls({ disabled, onSyncAll }: SyncControlsProps) {
  return (
    <Card title={syncControlsTitle}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <Alert showIcon={true} type="info" message={syncInfoMessage} />
        <Button
          disabled={disabled}
          icon={<PlayCircleOutlined />}
          type="primary"
          onClick={onSyncAll}
        >
          {syncAllButtonLabel}
        </Button>
      </Space>
    </Card>
  );
}
