import { SaveOutlined } from "@ant-design/icons";
import {
  Alert,
  App,
  AutoComplete,
  Button,
  Card,
  Checkbox,
  Form,
  Input,
  InputNumber,
  Select,
  Space,
  Typography,
} from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "../api.ts";
import { useI18n } from "../i18n.tsx";
import type { ConfigResponse, UpdateConfigRequest } from "../types.ts";

interface ConfigFormProps {
  config: ConfigResponse | null;
  mode?: "settings" | "onboarding";
  onSaved: (config: ConfigResponse, apiKey?: string) => void;
}

interface ConfigFormValues {
  username: string;
  password?: string;
  formats: string[];
  outputDir: string;
  directoryTemplate: string;
  skipExisting: boolean;
  singleThreaded: boolean;
  maxConcurrentAlbums: number;
  generateReadme: boolean;
  generateNfo: boolean;
  debug: boolean;
  metadataOnly: boolean;
  scheduleEnabled: boolean;
  scheduleCron: string;
  apiKey?: string;
}

export function ConfigForm({ config, mode = "settings", onSaved }: ConfigFormProps) {
  const { message } = App.useApp();
  const { t } = useI18n();
  const [form] = Form.useForm<ConfigFormValues>();
  const [saving, setSaving] = useState(false);
  const isOnboarding = mode === "onboarding";
  const outputDirLocked = Boolean(config?.config.paths.output_dir_locked);

  const formatOptions = useMemo(
    () => [
      { label: "128kbps MP3", value: "128" },
      { label: "320kbps MP3", value: "320" },
      { label: "FLAC", value: "FLAC" },
      { label: "gift", value: "gift" },
    ],
    [],
  );

  const templateOptions = useMemo(
    () => [
      { label: t("config.template.default"), value: "{album}/@{label}" },
      { label: t("config.template.labelAlbum"), value: "@{label}/{album}" },
      { label: t("config.template.yearAlbum"), value: "{year}/{album}" },
      { label: t("config.template.artistAlbum"), value: "{authors}/{album}" },
      { label: t("config.template.dateAlbum"), value: "{date} - {album}" },
    ],
    [t],
  );

  const initialValues = useMemo<ConfigFormValues | undefined>(() => {
    if (!config) {
      return;
    }

    return {
      username: config.config.user.username,
      password: "",
      formats: config.config.download.formats,
      outputDir: config.config.paths.output_dir,
      directoryTemplate: config.config.paths.directory_template,
      skipExisting: config.config.behavior.skip_existing,
      singleThreaded: config.config.behavior.single_threaded,
      maxConcurrentAlbums: config.config.behavior.max_concurrent_albums || 1,
      generateReadme: config.config.behavior.generate_readme,
      generateNfo: config.config.behavior.generate_nfo,
      debug: config.config.behavior.debug,
      metadataOnly: config.config.behavior.metadata_only,
      scheduleEnabled: config.config.schedule.enabled,
      scheduleCron: config.config.schedule.cron,
      apiKey: "",
    };
  }, [config]);

  useEffect(() => {
    if (initialValues) {
      form.setFieldsValue(initialValues);
    }
  }, [form, initialValues]);

  const submit = useCallback(
    async (values: ConfigFormValues) => {
      if (values.formats.includes("128") && values.formats.includes("320")) {
        message.error(t("config.formatConflict"));
        return;
      }

      const password = values.password?.trim();
      const apiKey = values.apiKey?.trim();
      const payload: UpdateConfigRequest = {
        user: {
          username: values.username.trim(),
          ...(password ? { password } : {}),
        },
        download: {
          formats: values.formats,
        },
        paths: {
          ...(outputDirLocked ? {} : { output_dir: values.outputDir.trim() }),
          directory_template: values.directoryTemplate.trim(),
        },
        behavior: {
          skip_existing: values.skipExisting,
          single_threaded: values.singleThreaded,
          max_concurrent_albums: values.maxConcurrentAlbums || 1,
          generate_readme: values.generateReadme,
          generate_nfo: values.generateNfo,
          debug: values.debug,
          metadata_only: values.metadataOnly,
        },
        schedule: {
          enabled: values.scheduleEnabled,
          cron: values.scheduleCron.trim(),
        },
        api: apiKey ? { api_key: apiKey } : undefined,
      };

      setSaving(true);
      try {
        const nextConfig = await api.updateConfig(payload);
        message.success(t("config.saved"));
        form.setFieldValue("password", "");
        form.setFieldValue("apiKey", "");
        onSaved(nextConfig, apiKey || undefined);
      } catch (caught) {
        message.error(caught instanceof Error ? caught.message : String(caught));
      } finally {
        setSaving(false);
      }
    },
    [form, message, onSaved, outputDirLocked, t],
  );

  return (
    <Card title={isOnboarding ? t("config.onboardingTitle") : t("config.title")}>
      <Typography.Paragraph type="secondary">
        {t("config.description", { path: config?.config_path ?? t("config.unknown") })}
      </Typography.Paragraph>
      {outputDirLocked ? (
        <Alert
          showIcon={true}
          style={{ marginBottom: 16 }}
          type="info"
          message={t("config.outputDirLocked")}
        />
      ) : null}
      <Form form={form} layout="vertical" onFinish={submit}>
        <Space align="start" size="large" style={{ width: "100%" }} wrap={true}>
          <Form.Item
            label={t("config.username")}
            name="username"
            rules={[{ required: true, message: t("config.usernameRequired") }]}
          >
            <Input autoComplete="username" style={{ width: 280 }} />
          </Form.Item>
          <Form.Item
            label={t("config.password")}
            name="password"
            rules={[
              {
                required: isOnboarding && !config?.config.user.has_password,
                message: t("config.passwordRequired"),
              },
            ]}
          >
            <Input.Password
              autoComplete="current-password"
              placeholder={t("config.passwordPlaceholder")}
              style={{ width: 280 }}
            />
          </Form.Item>
          <Form.Item label={t("config.webPassword")} name="apiKey">
            <Input.Password
              placeholder={t("config.webPasswordPlaceholder")}
              style={{ width: 280 }}
            />
          </Form.Item>
        </Space>

        <Form.Item
          label={t("config.formats")}
          name="formats"
          rules={[{ required: true, message: t("config.formatsRequired") }]}
        >
          <Select mode="multiple" options={formatOptions} />
        </Form.Item>

        <Space align="start" size="large" style={{ width: "100%" }} wrap={true}>
          <Form.Item
            label={t("config.outputDir")}
            name="outputDir"
            rules={[{ required: true, message: t("config.outputDirRequired") }]}
            tooltip={outputDirLocked ? t("config.outputDirLocked") : undefined}
          >
            <Input disabled={outputDirLocked} style={{ width: 320 }} />
          </Form.Item>
          <Form.Item
            label={t("config.directoryTemplate")}
            name="directoryTemplate"
            rules={[{ required: true, message: t("config.directoryTemplateRequired") }]}
          >
            <AutoComplete
              options={templateOptions}
              filterOption={(inputValue, option) =>
                String(option?.label ?? "")
                  .toLowerCase()
                  .includes(inputValue.toLowerCase()) ||
                String(option?.value ?? "")
                  .toLowerCase()
                  .includes(inputValue.toLowerCase())
              }
              style={{ width: 360 }}
            />
          </Form.Item>
          <Form.Item
            label={t("config.maxConcurrentAlbums")}
            name="maxConcurrentAlbums"
            rules={[
              {
                required: true,
                type: "number",
                min: 1,
                message: t("config.maxConcurrentAlbumsRequired"),
              },
            ]}
          >
            <InputNumber min={1} style={{ width: 180 }} />
          </Form.Item>
        </Space>

        <Space size="large" wrap={true}>
          <Form.Item name="skipExisting" valuePropName="checked">
            <Checkbox>{t("config.skipExisting")}</Checkbox>
          </Form.Item>
          <Form.Item name="singleThreaded" valuePropName="checked">
            <Checkbox>{t("config.singleThreaded")}</Checkbox>
          </Form.Item>
          <Form.Item name="generateReadme" valuePropName="checked">
            <Checkbox>{t("config.generateReadme")}</Checkbox>
          </Form.Item>
          <Form.Item name="generateNfo" valuePropName="checked">
            <Checkbox>{t("config.generateNfo")}</Checkbox>
          </Form.Item>
          <Form.Item name="metadataOnly" valuePropName="checked">
            <Checkbox>{t("config.metadataOnly")}</Checkbox>
          </Form.Item>
          <Form.Item name="debug" valuePropName="checked">
            <Checkbox>{t("config.debug")}</Checkbox>
          </Form.Item>
        </Space>

        <Space align="start" size="large" style={{ width: "100%" }} wrap={true}>
          <Form.Item name="scheduleEnabled" valuePropName="checked">
            <Checkbox>{t("config.scheduleEnabled")}</Checkbox>
          </Form.Item>
          <Form.Item
            label={t("config.scheduleCron")}
            name="scheduleCron"
            rules={[{ required: true, message: t("config.scheduleCronRequired") }]}
            tooltip={t("config.scheduleCronHelp")}
          >
            <Input placeholder="0 0 3 * * * *" style={{ width: 320 }} />
          </Form.Item>
        </Space>

        <Form.Item>
          <Button htmlType="submit" icon={<SaveOutlined />} loading={saving} type="primary">
            {isOnboarding ? t("config.saveOnboarding") : t("config.save")}
          </Button>
        </Form.Item>
      </Form>
    </Card>
  );
}
