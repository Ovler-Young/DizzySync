import { SaveOutlined } from "@ant-design/icons";
import {
  App,
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
import { useEffect, useMemo, useState } from "react";
import { api } from "../api.ts";
import type { ConfigResponse, UpdateConfigRequest } from "../types.ts";

interface ConfigFormProps {
  config: ConfigResponse | null;
  onSaved: (config: ConfigResponse) => void;
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
  apiKey?: string;
}

const formatOptions = [
  { label: "128kbps MP3", value: "128" },
  { label: "320kbps MP3", value: "320" },
  { label: "FLAC", value: "FLAC" },
  { label: "特典 gift", value: "gift" },
];

export default function ConfigForm({ config, onSaved }: ConfigFormProps) {
  const { message } = App.useApp();
  const [form] = Form.useForm<ConfigFormValues>();
  const [saving, setSaving] = useState(false);

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
      maxConcurrentAlbums: config.config.behavior.max_concurrent_albums,
      generateReadme: config.config.behavior.generate_readme,
      generateNfo: config.config.behavior.generate_nfo,
      debug: config.config.behavior.debug,
      metadataOnly: config.config.behavior.metadata_only,
      apiKey: "",
    };
  }, [config]);

  useEffect(() => {
    if (initialValues) {
      form.setFieldsValue(initialValues);
    }
  }, [form, initialValues]);

  const submit = async (values: ConfigFormValues) => {
    if (values.formats.includes("128") && values.formats.includes("320")) {
      message.error("128 和 320 不能同时选择，因为都会输出 .mp3 文件");
      return;
    }

    const password = values.password?.trim();
    const apiKey = values.apiKey?.trim();
    const payload: UpdateConfigRequest = {
      user: {
        username: values.username,
        ...(password ? { password } : {}),
      },
      download: {
        formats: values.formats,
      },
      paths: {
        output_dir: values.outputDir,
        directory_template: values.directoryTemplate,
      },
      behavior: {
        skip_existing: values.skipExisting,
        single_threaded: values.singleThreaded,
        max_concurrent_albums: values.maxConcurrentAlbums,
        generate_readme: values.generateReadme,
        generate_nfo: values.generateNfo,
        debug: values.debug,
        metadata_only: values.metadataOnly,
      },
      api: apiKey ? { api_key: apiKey } : undefined,
    };

    setSaving(true);
    try {
      const nextConfig = await api.updateConfig(payload);
      message.success("配置已保存到 TOML");
      form.setFieldValue("password", "");
      form.setFieldValue("apiKey", "");
      onSaved(nextConfig);
    } catch (caught) {
      message.error(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card title="设置">
      <Typography.Paragraph type="secondary">
        配置文件：{config?.config_path ?? "未知"}。密码和 API Key 留空表示保持当前值。
      </Typography.Paragraph>
      <Form form={form} layout="vertical" onFinish={submit}>
        <Space align="start" size="large" style={{ width: "100%" }} wrap={true}>
          <Form.Item
            label="Dizzylab 用户名"
            name="username"
            rules={[{ required: true, message: "请输入用户名" }]}
          >
            <Input autoComplete="username" style={{ width: 280 }} />
          </Form.Item>
          <Form.Item label="Dizzylab 密码" name="password">
            <Input.Password
              autoComplete="current-password"
              placeholder="留空保持不变"
              style={{ width: 280 }}
            />
          </Form.Item>
          <Form.Item label="API Key" name="apiKey">
            <Input.Password placeholder="留空保持不变" style={{ width: 280 }} />
          </Form.Item>
        </Space>

        <Form.Item
          label="下载格式"
          name="formats"
          rules={[{ required: true, message: "请选择至少一种格式" }]}
        >
          <Select mode="multiple" options={formatOptions} />
        </Form.Item>

        <Space align="start" size="large" style={{ width: "100%" }} wrap={true}>
          <Form.Item
            label="输出目录"
            name="outputDir"
            rules={[{ required: true, message: "请输入输出目录" }]}
          >
            <Input style={{ width: 320 }} />
          </Form.Item>
          <Form.Item
            label="目录模板"
            name="directoryTemplate"
            rules={[{ required: true, message: "请输入目录模板" }]}
          >
            <Input style={{ width: 320 }} />
          </Form.Item>
          <Form.Item label="最大并发专辑数" name="maxConcurrentAlbums">
            <InputNumber min={1} style={{ width: 180 }} />
          </Form.Item>
        </Space>

        <Space size="large" wrap={true}>
          <Form.Item name="skipExisting" valuePropName="checked">
            <Checkbox>跳过已存在目录</Checkbox>
          </Form.Item>
          <Form.Item name="singleThreaded" valuePropName="checked">
            <Checkbox>单线程</Checkbox>
          </Form.Item>
          <Form.Item name="generateReadme" valuePropName="checked">
            <Checkbox>生成 README</Checkbox>
          </Form.Item>
          <Form.Item name="generateNfo" valuePropName="checked">
            <Checkbox>生成 NFO</Checkbox>
          </Form.Item>
          <Form.Item name="metadataOnly" valuePropName="checked">
            <Checkbox>仅元数据</Checkbox>
          </Form.Item>
          <Form.Item name="debug" valuePropName="checked">
            <Checkbox>调试日志</Checkbox>
          </Form.Item>
        </Space>

        <Form.Item>
          <Button htmlType="submit" icon={<SaveOutlined />} loading={saving} type="primary">
            保存配置
          </Button>
        </Form.Item>
      </Form>
    </Card>
  );
}
