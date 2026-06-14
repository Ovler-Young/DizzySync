import type { Locale } from "antd/es/locale";
import enUs from "antd/locale/en_US";
import zhCn from "antd/locale/zh_CN";
import type { ReactNode } from "react";
import { createContext, useCallback, useContext, useMemo, useState } from "react";

export type Language = "zh-CN" | "en-US";

type TranslationValue = string | ((params: Record<string, string | number>) => string);
type TranslationMap = Record<string, TranslationValue>;

const translations: Record<Language, TranslationMap> = {
  "zh-CN": {
    "app.title": "DizzySync 控制台",
    "app.heading": "音乐同步与配置管理",
    "app.subtitle":
      "通过 Web UI 完成首次设置、管理 Dizzylab 同步配置，并触发专辑同步。Docker 部署只需要在环境变量中提供 Web UI 密码。",
    "app.refresh": "刷新",
    "app.apiKey.placeholder": "Web UI 密码",
    "app.error.title": "请求失败",
    "app.language": "语言",
    "auth.required": "需要 Web UI 密码",
    "auth.requiredDescription":
      "请输入 Docker 环境变量 DIZZYSYNC_WEB_PASSWORD 中设置的 Web UI 密码。",
    "auth.loginTitle": "登录 DizzySync",
    "auth.login": "登录",
    "tabs.dashboard": "控制台",
    "tabs.logs": "日志",
    "tabs.settings": "设置",
    "tabs.onboarding": "开始使用",
    "onboarding.title": "首次设置",
    "onboarding.welcome":
      "欢迎使用 DizzySync。请先填写 Dizzylab 登录信息并确认同步设置；保存前会验证凭据，验证成功后才会写入服务端配置。",
    "onboarding.notReady": "服务尚未就绪",
    "onboarding.notReadyDescription":
      "请完成首次设置。Dizzylab 用户名/密码、下载格式、输出路径和同步行为均可在 Web UI 中维护。",
    "status.loading": "正在读取服务状态...",
    "status.title": "服务状态",
    "status.api": "API 状态",
    "status.login": "登录状态",
    "status.ready": "已就绪",
    "status.notReady": "未就绪",
    "status.user": "账号",
    "status.syncJob": "同步任务",
    "status.idle": "空闲",
    "status.lastError": "最近错误：{message}",
    "sync.title": "同步控制",
    "sync.info": "同一时间只允许一个同步任务运行。任务启动后可在状态区域查看运行状态。",
    "sync.all": "同步全部已购专辑",
    "album.title": "已购专辑",
    "album.cover": "封面",
    "album.name": "标题",
    "album.label": "厂牌",
    "album.actions": "操作",
    "album.detail": "详情",
    "album.sync": "同步",
    "album.reload": "重新加载",
    "detail.title": "专辑详情",
    "detail.sync": "同步此专辑",
    "detail.releaseDate": "发布日期",
    "detail.gift": "特典",
    "detail.hasGift": "有",
    "detail.noGift": "无",
    "detail.tags": "标签",
    "detail.tracks": "曲目 ({count})",
    "config.title": "设置",
    "config.onboardingTitle": "首次设置",
    "config.description": "配置文件：{path}。Dizzylab 账号密码和 Web UI 密码留空表示保持当前值。",
    "config.unknown": "未知",
    "config.username": "Dizzylab 用户名",
    "config.usernameRequired": "请输入 Dizzylab 用户名",
    "config.password": "Dizzylab 密码",
    "config.passwordRequired": "请输入 Dizzylab 密码",
    "config.passwordPlaceholder": "留空保持不变",
    "config.addAccount": "添加 Dizzylab 账号",
    "config.removeAccount": "移除账号",
    "config.webPassword": "Web UI 密码",
    "config.webPasswordRequired": "请输入 Web UI 密码",
    "config.webPasswordPlaceholder": "留空保持不变",
    "config.formats": "下载格式",
    "config.formatsRequired": "请选择至少一种格式",
    "config.formatConflict": "128 和 320 不能同时选择，因为都会输出 .mp3 文件",
    "config.outputDir": "输出目录",
    "config.outputDirRequired": "请输入输出目录",
    "config.outputDirLocked": "输出目录由 DIZZYSYNC_OUTPUT_DIR 自动写入，Web UI 中不允许修改。",
    "config.directoryTemplate": "目录模板",
    "config.directoryTemplateRequired": "请输入目录模板",
    "config.template.default": "推荐：专辑名 / @厂牌名",
    "config.template.labelAlbum": "按厂牌归档：@厂牌名 / 专辑名",
    "config.template.yearAlbum": "按年份归档：年份 / 专辑名",
    "config.template.artistAlbum": "按作者归档：作者 / 专辑名",
    "config.template.dateAlbum": "按日期命名：日期 - 专辑名",
    "config.maxConcurrentAlbums": "最大并发专辑数",
    "config.maxConcurrentAlbumsRequired": "请输入不小于 1 的并发数",
    "config.skipExisting": "跳过已存在目录",
    "config.singleThreaded": "单线程",
    "config.generateReadme": "生成 README",
    "config.generateNfo": "生成 NFO",
    "config.metadataOnly": "仅元数据",
    "config.debug": "调试日志",
    "config.save": "保存配置",
    "config.saveOnboarding": "验证并完成设置",
    "config.saved": "配置已验证并保存",
    "logs.title": "日志查看",
    "logs.description": "显示 Web API 和同步任务的最近运行日志，便于排查登录、配置和下载问题。",
    "logs.refresh": "刷新日志",
    "logs.empty": "暂无日志",
    "guide.title": "配置指南",
    "guide.credentials.title": "凭据保存位置",
    "guide.credentials.description":
      "Dizzylab 凭据和同步配置保存在服务端 config.toml 中；Docker Compose 默认将该文件放在 dizzysync_config 卷中。Docker 环境变量只需要提供 Web UI 密码。",
    "guide.user.label": "登录凭据 [[users]]",
    "guide.user.body":
      "可配置一个或多个 Dizzylab 账号。username 和 password 是登录凭据；首次设置和后续修改均在 Web UI 中完成。保存时会逐个登录 Dizzylab 验证，全部成功后才写入配置。旧版 [user] 配置仍会被兼容读取。",
    "guide.download.label": "下载格式 [download]",
    "guide.download.conflict": "128 和 320 都会输出 .mp3 文件，不能同时选择，否则文件名会冲突。",
    "guide.paths.label": "路径与目录模板 [paths]",
    "guide.paths.body":
      "output_dir 是下载输出目录；设置 DIZZYSYNC_OUTPUT_DIR 后会自动写入并锁定，Web UI 不允许修改。directory_template 支持变量：{album}、{label}、{authors}、{year}、{date}；建议优先使用预设模板，避免文件名冲突。",
    "guide.behavior.label": "同步行为 [behavior]",
    "guide.behavior.skipExisting": "skip_existing：跳过已存在目录。",
    "guide.behavior.singleThreaded": "single_threaded：单线程下载，减轻服务器压力。",
    "guide.behavior.maxConcurrent": "max_concurrent_albums：关闭单线程后同时处理的专辑数。",
    "guide.behavior.metadata":
      "generate_readme / generate_nfo：生成媒体库元数据文件。metadata_only：只下载封面、README、NFO，不下载音频。",
    "guide.behavior.debug": "debug：输出更详细的 HTTP 调试日志。",
    "guide.api.label": "API 与 Web 控制 [api]",
    "guide.api.body":
      "Rust 服务同时提供 Web GUI 和 /api/*。Docker 监听 0.0.0.0:8787，但只暴露一个端口。Web UI 密码会作为 API key 保存到配置中，用于保护远程控制接口。",
  },
  "en-US": {
    "app.title": "DizzySync Console",
    "app.heading": "Music Sync and Configuration",
    "app.subtitle":
      "Complete onboarding, manage Dizzylab sync settings, and start album syncs from the Web UI. Docker only needs the Web UI password as an environment variable.",
    "app.refresh": "Refresh",
    "app.apiKey.placeholder": "Web UI password",
    "app.error.title": "Request failed",
    "app.language": "Language",
    "auth.required": "Web UI password required",
    "auth.requiredDescription":
      "Enter the Web UI password configured with the DIZZYSYNC_WEB_PASSWORD Docker environment variable.",
    "auth.loginTitle": "Log in to DizzySync",
    "auth.login": "Log in",
    "tabs.dashboard": "Dashboard",
    "tabs.logs": "Logs",
    "tabs.settings": "Settings",
    "tabs.onboarding": "Get started",
    "onboarding.title": "First-time setup",
    "onboarding.welcome":
      "Welcome to DizzySync. Enter your Dizzylab credentials and review sync settings. Credentials are validated before anything is saved.",
    "onboarding.notReady": "Service is not ready",
    "onboarding.notReadyDescription":
      "Complete first-time setup. Dizzylab credentials, formats, paths, and sync behavior are all managed in the Web UI.",
    "status.loading": "Loading service status...",
    "status.title": "Service status",
    "status.api": "API status",
    "status.login": "Login status",
    "status.ready": "Ready",
    "status.notReady": "Not ready",
    "status.user": "Accounts",
    "status.syncJob": "Sync job",
    "status.idle": "Idle",
    "status.lastError": "Last error: {message}",
    "sync.title": "Sync controls",
    "sync.info": "Only one sync job can run at a time. Watch the status card after a job starts.",
    "sync.all": "Sync all purchased albums",
    "album.title": "Purchased albums",
    "album.cover": "Cover",
    "album.name": "Title",
    "album.label": "Label",
    "album.actions": "Actions",
    "album.detail": "Details",
    "album.sync": "Sync",
    "album.reload": "Reload",
    "detail.title": "Album details",
    "detail.sync": "Sync this album",
    "detail.releaseDate": "Release date",
    "detail.gift": "Gift",
    "detail.hasGift": "Yes",
    "detail.noGift": "No",
    "detail.tags": "Tags",
    "detail.tracks": "Tracks ({count})",
    "config.title": "Settings",
    "config.onboardingTitle": "First-time setup",
    "config.description":
      "Config file: {path}. Leave Dizzylab account passwords or the Web UI password empty to keep current values.",
    "config.unknown": "Unknown",
    "config.username": "Dizzylab username",
    "config.usernameRequired": "Enter your Dizzylab username",
    "config.password": "Dizzylab password",
    "config.passwordRequired": "Enter your Dizzylab password",
    "config.passwordPlaceholder": "Leave blank to keep current value",
    "config.addAccount": "Add Dizzylab account",
    "config.removeAccount": "Remove account",
    "config.webPassword": "Web UI password",
    "config.webPasswordRequired": "Enter the Web UI password",
    "config.webPasswordPlaceholder": "Leave blank to keep current value",
    "config.formats": "Download formats",
    "config.formatsRequired": "Select at least one format",
    "config.formatConflict": "128 and 320 cannot both be selected because both write .mp3 files",
    "config.outputDir": "Output directory",
    "config.outputDirRequired": "Enter the output directory",
    "config.outputDirLocked":
      "The output directory is written from DIZZYSYNC_OUTPUT_DIR and cannot be changed in the Web UI.",
    "config.directoryTemplate": "Directory template",
    "config.directoryTemplateRequired": "Enter the directory template",
    "config.template.default": "Recommended: album / @label",
    "config.template.labelAlbum": "Group by label: @label / album",
    "config.template.yearAlbum": "Group by year: year / album",
    "config.template.artistAlbum": "Group by artist: artist / album",
    "config.template.dateAlbum": "Date prefix: date - album",
    "config.maxConcurrentAlbums": "Max concurrent albums",
    "config.maxConcurrentAlbumsRequired": "Enter a value no less than 1",
    "config.skipExisting": "Skip existing directories",
    "config.singleThreaded": "Single threaded",
    "config.generateReadme": "Generate README",
    "config.generateNfo": "Generate NFO",
    "config.metadataOnly": "Metadata only",
    "config.debug": "Debug logs",
    "config.save": "Save configuration",
    "config.saveOnboarding": "Validate and finish setup",
    "config.saved": "Configuration validated and saved",
    "logs.title": "Log viewer",
    "logs.description":
      "Shows recent Web API and sync job logs for troubleshooting login, configuration, and downloads.",
    "logs.refresh": "Refresh logs",
    "logs.empty": "No logs yet",
    "guide.title": "Configuration guide",
    "guide.credentials.title": "Where credentials are stored",
    "guide.credentials.description":
      "Dizzylab credentials and sync settings are stored in the server-side config.toml. Docker Compose stores it in the dizzysync_config volume by default. Docker only needs the Web UI password as an environment variable.",
    "guide.user.label": "Login credentials [[users]]",
    "guide.user.body":
      "Configure one or more Dizzylab accounts. username and password are the login credentials. Initial setup and future changes happen in the Web UI. Saving validates every account against Dizzylab before writing config. Legacy [user] config is still read for compatibility.",
    "guide.download.label": "Download formats [download]",
    "guide.download.conflict":
      "128 and 320 both output .mp3 files and cannot be selected together.",
    "guide.paths.label": "Paths and directory template [paths]",
    "guide.paths.body":
      "output_dir is the download directory. When DIZZYSYNC_OUTPUT_DIR is set it is written automatically and locked in the Web UI. directory_template supports {album}, {label}, {authors}, {year}, and {date}; prefer the presets to avoid filename conflicts.",
    "guide.behavior.label": "Sync behavior [behavior]",
    "guide.behavior.skipExisting": "skip_existing: skip directories that already exist.",
    "guide.behavior.singleThreaded":
      "single_threaded: download one album at a time to reduce server pressure.",
    "guide.behavior.maxConcurrent":
      "max_concurrent_albums: number of albums processed at once when single-threaded mode is off.",
    "guide.behavior.metadata":
      "generate_readme / generate_nfo: generate media-library metadata. metadata_only: download covers, README, and NFO only, not audio.",
    "guide.behavior.debug": "debug: print more detailed HTTP debug logs.",
    "guide.api.label": "API and Web control [api]",
    "guide.api.body":
      "The Rust service provides both the Web GUI and /api/*. Docker listens on 0.0.0.0:8787 and exposes only one port. The Web UI password is saved as the API key to protect remote control access.",
  },
};

interface I18nContextValue {
  language: Language;
  antdLocale: Locale;
  setLanguage: (language: Language) => void;
  t: (key: string, params?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);
const languageStorageKey = "dizzysync.language";

function detectInitialLanguage(): Language {
  const saved = globalThis.localStorage?.getItem(languageStorageKey);
  return saved === "en-US" ? "en-US" : "zh-CN";
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<Language>(detectInitialLanguage);

  const setLanguage = useCallback((nextLanguage: Language) => {
    setLanguageState(nextLanguage);
    globalThis.localStorage?.setItem(languageStorageKey, nextLanguage);
  }, []);

  const t = useCallback(
    (key: string, params: Record<string, string | number> = {}) => {
      const value = translations[language][key] ?? translations["zh-CN"][key] ?? key;
      if (typeof value === "function") {
        return value(params);
      }
      return value.replace(/\{(\w+)\}/g, (_, name: string) => String(params[name] ?? `{${name}}`));
    },
    [language],
  );

  const value = useMemo<I18nContextValue>(
    () => ({
      language,
      antdLocale: language === "zh-CN" ? zhCn : enUs,
      setLanguage,
      t,
    }),
    [language, setLanguage, t],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error("useI18n must be used within I18nProvider");
  }
  return context;
}
