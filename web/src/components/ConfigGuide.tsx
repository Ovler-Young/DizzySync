import { Alert, Card, Collapse, Typography } from "antd";

const { Paragraph, Text } = Typography;

export default function ConfigGuide() {
  return (
    <Card title="配置指南">
      <Alert
        showIcon={true}
        style={{ marginBottom: 16 }}
        type="warning"
        message="凭据保存位置"
        description="用户名、密码和 API Key 会保存在服务端的 config.toml 中。Docker Compose 默认将该文件放在 dizzysync_config 卷中。"
      />
      <Collapse
        items={[
          {
            key: "user",
            label: "登录凭据 [user]",
            children: (
              <Paragraph>
                <Text strong={true}>username</Text> 和 <Text strong={true}>password</Text> 是
                Dizzylab 登录凭据。 Docker Compose
                首次启动时只需要通过环境变量填写这两个值，服务会自动生成 TOML；之后可在本页面修改。
                保存设置时密码留空表示保持原密码不变。
              </Paragraph>
            ),
          },
          {
            key: "download",
            label: "下载格式 [download]",
            children: (
              <ul className="guide-list">
                <li>128：128kbps MP3。</li>
                <li>320：320kbps MP3。</li>
                <li>FLAC：无损格式。</li>
                <li>gift：特典内容。</li>
                <li>128 和 320 都会输出 .mp3 文件，不能同时选择，否则文件名会冲突。</li>
              </ul>
            ),
          },
          {
            key: "paths",
            label: "路径与目录模板 [paths]",
            children: (
              <Paragraph>
                <Text strong={true}>output_dir</Text> 是下载输出目录；Docker 中应保持为{" "}
                <Text code={true}>/data</Text>。<Text strong={true}> directory_template</Text>{" "}
                支持变量：<Text code={true}>{"{album}"}</Text>、<Text code={true}>{"{label}"}</Text>
                、<Text code={true}>{"{authors}"}</Text>、<Text code={true}>{"{year}"}</Text>、
                <Text code={true}>{"{date}"}</Text>。默认模板会按 “专辑名/@厂牌名” 组织文件。
              </Paragraph>
            ),
          },
          {
            key: "behavior",
            label: "同步行为 [behavior]",
            children: (
              <ul className="guide-list">
                <li>skip_existing：跳过已存在目录。</li>
                <li>single_threaded：单线程下载，减轻服务器压力。</li>
                <li>max_concurrent_albums：关闭单线程后同时处理的专辑数。</li>
                <li>generate_readme / generate_nfo：生成媒体库元数据文件。</li>
                <li>metadata_only：只下载封面、README、NFO，不下载音频。</li>
                <li>debug：输出更详细的 HTTP 调试日志。</li>
              </ul>
            ),
          },
          {
            key: "api",
            label: "API 与 Web 控制 [api]",
            children: (
              <Paragraph>
                Rust 服务同时提供 Web GUI 和 <Text code={true}>/api/*</Text>。本地默认监听
                <Text code={true}>127.0.0.1:8787</Text>；Docker 会监听{" "}
                <Text code={true}>0.0.0.0:8787</Text>， 但只暴露一个端口。如果非本地监听且未设置 API
                Key，服务会自动生成一个并写入配置， 防止远程无认证控制。
              </Paragraph>
            ),
          },
        ]}
      />
    </Card>
  );
}
