import {
  ApiOutlined,
  CheckCircleOutlined,
  ClockCircleOutlined,
  CloudSyncOutlined,
  ExclamationCircleOutlined,
  LoginOutlined,
  UserOutlined,
} from "@ant-design/icons";
import { Alert, Card, Tag, Typography } from "antd";
import type { ReactNode } from "react";
import { useI18n } from "../i18n.tsx";
import type { StatusResponse, UserInfo } from "../types.ts";

interface StatusCardProps {
  status: StatusResponse | null;
}

interface StatusMetricProps {
  icon: ReactNode;
  label: string;
  value: ReactNode;
  meta?: ReactNode;
  tone?: "green" | "blue" | "orange" | "default";
}

interface StatusChipProps {
  icon?: ReactNode;
  label: string;
  value: ReactNode;
  tone?: "green" | "blue" | "orange" | "default";
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

function StatusMetric({ icon, label, value, meta, tone = "default" }: StatusMetricProps) {
  return (
    <div className={`status-metric status-metric-${tone}`}>
      <div className="status-metric-icon">{icon}</div>
      <div className="status-metric-body">
        <Typography.Text className="status-metric-label" type="secondary">
          {label}
        </Typography.Text>
        <div className="status-metric-value">{value}</div>
        {meta ? <div className="status-metric-meta">{meta}</div> : null}
      </div>
    </div>
  );
}

function StatusChip({ icon, label, value, tone = "default" }: StatusChipProps) {
  return (
    <div className={`status-chip status-chip-${tone}`}>
      {icon ? <span className="status-chip-icon">{icon}</span> : null}
      <span className="status-chip-label">{label}</span>
      <span className="status-chip-value">{value}</span>
    </div>
  );
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
  let loginTone: StatusMetricProps["tone"] = "orange";
  if (status.ready) {
    loginColor = "green";
    loginText = t("status.ready");
    loginTone = "green";
  }

  const users = resolveUsers(status);
  const accountText = users.length > 0 ? t("status.accountCount", { count: users.length }) : "-";
  const userText =
    users.length > 0 ? users.map((user) => `${user.username} (${user.uid})`).join(", ") : "-";

  let scheduleColor = "default";
  let scheduleText = t("status.scheduleDisabled");
  let scheduleCron = "-";
  let scheduleTone: StatusMetricProps["tone"] = "default";
  if (status.schedule.enabled) {
    scheduleColor = "blue";
    scheduleText = t("status.scheduleEnabled");
    scheduleCron = status.schedule.cron;
    scheduleTone = "blue";
  }

  const formatTime = (timestamp: number | null) => {
    if (!timestamp) {
      return "-";
    }
    return new Date(timestamp * 1000).toLocaleString();
  };

  const heroTone = status.ready ? "ready" : "attention";
  const heroIcon = status.ready ? <CheckCircleOutlined /> : <ExclamationCircleOutlined />;
  const heroTitle = status.ready ? t("status.heroReadyTitle") : t("status.heroNotReadyTitle");
  const heroDescription = status.ready
    ? t("status.heroReadyDescription")
    : t(status.requires_auth ? "status.heroAuthDescription" : "status.heroSetupDescription");
  const jobMeta =
    status.job.state === "running"
      ? `${t("status.startedAt")}: ${formatTime(status.job.started_at)}`
      : t("status.noActiveJob");

  return (
    <Card className="status-card" title={t("status.title")}>
      <div className={`status-hero status-hero-${heroTone}`}>
        <div className="status-hero-icon">{heroIcon}</div>
        <div className="status-hero-content">
          <Typography.Title className="status-hero-title" level={4}>
            {heroTitle}
          </Typography.Title>
          <Typography.Text className="status-hero-description">{heroDescription}</Typography.Text>
          <div className="status-hero-chips">
            <StatusChip
              icon={<ApiOutlined />}
              label={t("status.api")}
              tone="green"
              value={<Tag color="green">{status.status}</Tag>}
            />
            <StatusChip
              icon={<LoginOutlined />}
              label={t("status.login")}
              tone={loginTone}
              value={<Tag color={loginColor}>{loginText}</Tag>}
            />
            <StatusChip
              icon={<CloudSyncOutlined />}
              label={t("status.syncJob")}
              tone={status.job.state === "running" ? "blue" : "default"}
              value={<Tag color={jobColor}>{job}</Tag>}
            />
          </div>
        </div>
      </div>

      <div className="status-metric-grid">
        <StatusMetric
          icon={<UserOutlined />}
          label={t("status.user")}
          meta={<Typography.Text ellipsis={{ tooltip: userText }}>{userText}</Typography.Text>}
          value={accountText}
        />
        <StatusMetric
          icon={<CloudSyncOutlined />}
          label={t("status.syncJob")}
          meta={jobMeta}
          tone={status.job.state === "running" ? "blue" : "default"}
          value={<Tag color={jobColor}>{job}</Tag>}
        />
        <StatusMetric
          icon={<ClockCircleOutlined />}
          label={t("status.schedule")}
          meta={scheduleCron}
          tone={scheduleTone}
          value={<Tag color={scheduleColor}>{scheduleText}</Tag>}
        />
        <StatusMetric
          icon={<ClockCircleOutlined />}
          label={t("status.nextRun")}
          meta={`${t("status.lastRun")}: ${formatTime(status.schedule.last_run)}`}
          value={formatTime(status.schedule.next_run)}
        />
      </div>

      <div className="status-detail-panel">
        <div className="status-detail-header">{t("status.accountSecurity")}</div>
        <div className="status-detail-content">
          <StatusChip
            label={t("status.accounts")}
            tone={users.length > 0 ? "green" : "orange"}
            value={accountText}
          />
          <StatusChip
            label={t("status.credentials")}
            tone={status.ready ? "green" : "orange"}
            value={status.ready ? t("status.credentialsVerified") : t("status.credentialsRedacted")}
          />
          <StatusChip
            label={t("status.configuration")}
            tone={status.configured ? "green" : "orange"}
            value={status.configured ? t("status.configured") : t("status.notConfigured")}
          />
        </div>
        <Typography.Text className="status-detail-note" type="secondary">
          {users.length > 0 ? userText : t("status.noAccounts")}
        </Typography.Text>
      </div>

      {status.schedule.last_error ? (
        <Alert
          className="status-alert"
          showIcon={true}
          type="error"
          message={t("status.scheduleLastError", { message: status.schedule.last_error })}
        />
      ) : null}
      {status.last_error ? (
        <Alert
          className="status-alert"
          showIcon={true}
          type="error"
          message={t("status.lastError", { message: status.last_error })}
        />
      ) : null}
    </Card>
  );
}
