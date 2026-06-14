import { PlayCircleOutlined } from "@ant-design/icons";
import { Alert, Button, Card, Space } from "antd";
import { useI18n } from "../i18n.tsx";

interface SyncControlsProps {
  disabled: boolean;
  onSyncAll: () => void;
}

export function SyncControls({ disabled, onSyncAll }: SyncControlsProps) {
  const { t } = useI18n();

  return (
    <Card title={t("sync.title")}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <Alert showIcon={true} type="info" message={t("sync.info")} />
        <Button
          disabled={disabled}
          icon={<PlayCircleOutlined />}
          type="primary"
          onClick={onSyncAll}
        >
          {t("sync.all")}
        </Button>
      </Space>
    </Card>
  );
}
