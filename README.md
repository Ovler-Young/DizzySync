# DizzySync - Dizzylab 自动同步器

DizzySync 是一个用 Rust 编写的 Dizzylab 音乐同步工具。它可以下载已购买专辑，并提供同一个 Rust 进程托管的 HTTP API 与 Web 控制台，用于管理配置、浏览专辑和触发同步任务。

## 特性

- 🎵 同步已购买专辑，支持全量同步和指定专辑同步
- 🎛️ 支持 128kbps MP3、320kbps MP3、FLAC、特典内容
- 🧾 生成封面、README 与 NFO 元数据
- 🔧 TOML 配置文件，可通过 Web UI/API 生成和维护
- 🌐 内置 Web 控制台：React + TypeScript + Ant Design
- 🚀 单进程部署：Rust 同时提供 `/api/*` 与前端静态文件
- 🐳 Docker Compose 一键部署，仅需填写登录凭据
- 📦 CI 构建 Docker 镜像并发布到 GHCR

## 快速开始：Docker Compose（推荐）

Docker 部署只暴露一个端口。用户只需要在 `.env` 中填写 Dizzylab 登录凭据；首次启动时 DizzySync 会自动生成 `/config/config.toml`，之后可通过 Web UI/API 管理配置。

```bash
mkdir dizzysync && cd dizzysync
curl -fsSLO https://raw.githubusercontent.com/Ovler-Young/DizzySync/main/docker-compose.yml
curl -fsSLO https://raw.githubusercontent.com/Ovler-Young/DizzySync/main/.env.example
cp .env.example .env
```

编辑 `.env`：

```env
# Web UI/API 访问密码。Dizzylab 账号在首次打开 Web UI 后配置，可添加多个账号。
DIZZYSYNC_WEB_PASSWORD=change_me_to_a_long_random_value

DIZZYSYNC_PORT=8787
DIZZYSYNC_DATA_DIR=./DizzySync
```

启动：

```bash
docker compose pull
docker compose up -d
```

访问 Web 控制台：

```text
http://localhost:8787
```

在页面右上角输入 `.env` 中的 `DIZZYSYNC_WEB_PASSWORD` 后再操作。首次进入后可在 Web UI 中添加一个或多个 Dizzylab 账号。

### Compose 部署结构

- 镜像：`ghcr.io/ovler-young/dizzysync:latest`
- 端口：宿主 `${DIZZYSYNC_PORT:-8787}` → 容器 `8787`
- 配置卷：`dizzysync-config:/config`
- 下载目录：`${DIZZYSYNC_DATA_DIR:-./DizzySync}:/data`
- 容器内前端目录：`/app/web`
- 容器内配置文件：`/config/config.toml`

Compose 文件不会触发本地 Docker build；镜像由 GitHub Actions 构建并发布到 GHCR。

## 从源码运行

### 依赖

- Rust stable
- Node.js 24+
- pnpm 11+

### 构建前端

```bash
cd web
pnpm install --frozen-lockfile
pnpm build
cd ..
```

### 构建 Rust 二进制

```bash
cargo build --release
```

二进制位于 `target/release/dizzysync`。

## 使用方法

### 创建默认配置

```bash
./target/release/dizzysync --init
```

生成 `config.toml` 后，填写用户名、密码、下载格式和输出目录。

### CLI 同步

```bash
# 干运行：仅列出专辑，不下载
./target/release/dizzysync --dry-run

# 开始同步
./target/release/dizzysync

# 仅下载元数据（专辑信息、封面、README、NFO），不下载音频文件
./target/release/dizzysync --metadata-only

# 仅下载指定 ID 的专辑
./target/release/dizzysync --id SWQX-01

# 使用自定义配置文件
./target/release/dizzysync -c /path/to/config.toml

# 指定输出目录
./target/release/dizzysync --output-dir /path/to/music
```

### 启动 API 与 Web 控制台

```bash
./target/release/dizzysync \
  --api-server \
  --config config.toml \
  --api-bind 127.0.0.1:8787 \
  --web-root web/dist
```

常用参数：

- `--api-server`：启动 HTTP API 与 Web 控制台
- `--api-bind ADDR`：监听地址，例如 `0.0.0.0:8787`
- `--api-key KEY`：设置 API Key；设置后请求必须携带 `X-API-Key` 或 `Authorization: Bearer <key>`
- `--web-root DIR`：前端静态文件目录
- `-c, --config FILE`：配置文件路径

安全默认值：API 默认监听 `127.0.0.1:8787`。如果绑定到非本机地址且未设置 API Key，服务会自动生成 API Key 并保存到配置文件。

## Web 控制台

Web 控制台使用 React、TypeScript、Ant Design 与 Vite 构建，由 Rust 服务直接托管，不需要 Caddy、Nginx 或单独的前端服务器。

主要功能：

- 查看服务状态、已登录账号和当前同步任务
- 输入/保存 API Key
- 浏览已购专辑列表
- 查看专辑详情、曲目和标签
- 触发全量同步或指定专辑同步
- 编辑配置并保存到 TOML
- 查看内置配置指南

Dizzylab 账号密码和 API Key 在配置表单中留空表示保持原值；保存时不会把空字符串写入 TOML。可添加多个 Dizzylab 账号，服务会逐个登录并同步各账号已购专辑。

## HTTP API

所有 API 路由位于 `/api` 下。

认证方式（设置 API Key 后）：

```http
X-API-Key: your_api_key
```

或：

```http
Authorization: Bearer your_api_key
```

端点摘要：

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `GET` | `/api/health` | 健康检查 |
| `GET` | `/api/status` | 服务状态、已登录账号信息、当前同步任务 |
| `GET` | `/api/config` | 读取公开配置（密码/API Key 会脱敏） |
| `PUT` | `/api/config` | 更新配置并写入 TOML |
| `POST` | `/api/config/bootstrap` | 从环境变量/默认值引导配置 |
| `GET` | `/api/albums` | 获取所有已配置账号的已购专辑列表（按专辑 ID 去重） |
| `GET` | `/api/albums/{id}` | 获取指定专辑详情 |
| `POST` | `/api/sync` | 启动全量同步 |
| `POST` | `/api/sync/{id}` | 启动指定专辑同步 |

当前 API 同一时间只允许一个同步任务运行；如果已有任务在运行，新同步请求会返回冲突错误。

## 配置文件

配置示例见 [`config.example.toml`](config.example.toml)。主要配置段：

```toml
# 旧版单账号 [user] 仍兼容；新配置推荐使用 [[users]]，可重复添加多个账号。
[[users]]
username = "your_username_here"
password = "your_password_here"

# [[users]]
# username = "another_username_here"
# password = "another_password_here"

[download]
formats = ["320", "FLAC"]

[paths]
output_dir = "./DizzySync"
directory_template = "{album}/@{label}"

[behavior]
skip_existing = true
single_threaded = true
max_concurrent_albums = 1
generate_readme = true
generate_nfo = true
metadata_only = false

[api]
bind = "127.0.0.1:8787"
api_key = ""
web_root = "./web/dist"
```

### 下载格式

- `"128"`：128kbps MP3
- `"320"`：320kbps MP3
- `"FLAC"`：无损 FLAC
- `"gift"`：特典内容

注意：`"128"` 与 `"320"` 都会输出 `.mp3`，不能同时选择。

### 目录模板变量

- `{album}`：专辑名
- `{label}`：厂牌名
- `{authors}`：首曲目的作者名
- `{year}`：发布年份
- `{date}`：发布日期（YYYY-MM-DD）

示例：

```toml
[paths]
directory_template = "{year}/{label}/{album}"
```

### 文件结构示例

```text
DizzySync/
└─ Example Album/
   └─ @Example Label/
      ├─ 01 Track One.mp3
      ├─ 02 Track Two.mp3
      ├─ 01 Track One.flac
      ├─ 02 Track Two.flac
      ├─ gift/
      │  └─ bonus.zip（解压内容）
      ├─ cover.jpg
      ├─ README.md
      └─ album.nfo
```

## 开发与检查

前端检查：

```bash
pnpm --dir web install --frozen-lockfile
pnpm --dir web check
pnpm --dir web typecheck
pnpm --dir web build
```

Rust 检查：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

CI 会运行 Rust fmt/clippy/test、前端 Biome/typecheck/build，并使用 Docker Buildx 缓存构建镜像。`main` 分支和 tag 会发布镜像到 GHCR；pull request 只构建验证，不推送镜像。

## 故障排除

### 无法登录

- 检查 Web UI 中每个 Dizzylab 账号的用户名和密码是否正确
- 如果直接编辑 TOML，推荐使用一个或多个 `[[users]]` 配置块；旧版 `[user]` 仍兼容
- 尝试在 Web UI 中重新保存凭据

### Web UI 提示未授权

- 请在页面右上角输入 `.env` 中配置的 `DIZZYSYNC_WEB_PASSWORD`
- 直接运行二进制且绑定非本地地址时，自动生成的 API Key 可在配置文件 `[api].api_key` 中查看

### 下载失败

- 网络连接问题或 Dizzylab 临时不可用
- 某些专辑可能不支持所选格式
- 尝试仅下载元数据或指定专辑排查问题
- 查看容器日志：`docker compose logs -f dizzysync`

## 免责声明

此工具仅用于下载用户已合法购买的内容。请遵守 Dizzylab 的服务条款和版权法律。

## License

MIT License
