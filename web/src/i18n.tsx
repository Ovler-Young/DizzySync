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
    "common.yes": "是",
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
    "status.schedule": "自动同步",
    "status.scheduleEnabled": "已启用",
    "status.scheduleDisabled": "已关闭",
    "status.scheduleCron": "Cron 表达式",
    "status.nextRun": "下次运行",
    "status.lastRun": "上次运行",
    "status.scheduleLastError": "自动同步错误：{message}",
    "status.lastError": "最近错误：{message}",
    "status.accountCount": ({ count }) => `${count} 个账号`,
    "sync.title": "同步控制",
    "sync.info": "同一时间只允许一个同步任务运行。任务启动后可在状态区域查看运行状态。",
    "sync.all": "同步全部已购专辑",
    "album.title": "已购专辑",
    "album.cover": "封面",
    "album.name": "标题",
    "album.label": "厂牌",
    "album.actions": "操作",
    "album.local": "本地状态",
    "album.localDownloaded": "已下载",
    "album.localNotDownloaded": "未下载",
    "album.localPartial": "部分下载",
    "album.localPath": "本地路径",
    "album.detail": "详情",
    "album.sync": "同步",
    "album.reload": "重新加载",
    "album.columns": "列",
    "album.tableView": "表格",
    "album.cardView": "卡片",
    "album.releaseDate": "发行时间",
    "album.tracks": "曲目",
    "album.trackCount": ({ count }) => `${count} 首`,
    "album.downloadedTrackCount": ({ count }) => `已下载 ${count} 首`,
    "album.trackProgress": ({ downloaded, total }) => `${downloaded}/${total} 首`,
    "album.formats": "格式",
    "album.gift": "特典",
    "album.id": "ID",
    "detail.title": "专辑详情",
    "detail.sync": "同步此专辑",
    "detail.openInDizzylab": "在 Dizzylab 打开",
    "detail.openLocalFile": "打开本地文件",
    "detail.playTrack": "播放/选择",
    "detail.selectedTrack": "当前曲目",
    "detail.releaseDate": "发布日期",
    "detail.gift": "特典",
    "detail.hasGift": "有",
    "detail.noGift": "无",
    "detail.tags": "标签",
    "detail.localSummary": "本地文件",
    "detail.localSummaryValue": "{downloaded}/{expected} 首，音频文件 {audio} 个",
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
    "config.testLogin": "测试登录",
    "config.testLoginSuccess": "登录测试成功",
    "config.testLoginFailed": "登录测试失败",
    "config.testLoginSuccessUser": "已登录并获取到用户：{username} (UID: {uid})",
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
    "config.template.flat": "平铺：专辑名",
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
    "config.scheduleEnabled": "启用自动同步",
    "config.scheduleCron": "自动同步 Cron",
    "config.scheduleCronRequired": "请输入 cron 表达式",
    "config.scheduleCronHelp":
      "使用 7 段 cron：秒 分 时 日 月 周 年。例如 0 0 3 * * * * 表示每天 03:00 同步。",
    "config.save": "保存配置",
    "config.saveOnboarding": "验证并完成设置",
    "config.saved": "配置已验证并保存",
    "logs.title": "日志查看",
    "logs.description": "显示 Web API 和同步任务的最近运行日志，便于排查登录、配置和下载问题。",
    "logs.refresh": "刷新日志",
    "logs.empty": "暂无日志",
    "logs.filterDate": "按日期筛选",
    "logs.filterLevel": "按级别筛选",
    "logs.filterStart": "开始时间",
    "logs.filterEnd": "结束时间",
    "footer.disclaimer":
      "DizzySync 是非官方项目，与 Dizzylab 官方无隶属、背书或合作关系。请遵守 Dizzylab 服务条款与当地法律，仅同步你有权访问的内容。",
    "footer.source": "开源地址",
    "footer.dizzylab": "Dizzylab 官网",
    "player.title": "全局音频播放器",
    "player.empty": "从专辑详情中选择已下载的本地曲目",
    "player.play": "播放",
    "player.pause": "暂停",
    "player.previous": "上一首",
    "player.next": "下一首",
    "player.loopOff": "不循环",
    "player.loopOne": "单曲循环",
    "player.loopAll": "列表循环",
    "guide.title": "配置指南",
    "guide.user.label": "登录凭据 [[users]]",
    "guide.user.body":
      "可配置一个或多个 Dizzylab 账号。username 和 password 是登录凭据；首次设置和后续修改均在 Web UI 中完成。保存时会逐个登录 Dizzylab 验证，全部成功后才写入配置。旧版 [user] 配置仍会被兼容读取。",
    "guide.download.label": "下载格式 [download]",
    "guide.download.conflict": "128 和 320 都会输出 .mp3 文件，不能同时选择，否则文件名会冲突。",
    "guide.paths.label": "路径与目录模板 [paths]",
    "guide.paths.body":
      "output_dir 是下载输出目录；设置 DIZZYSYNC_OUTPUT_DIR 后会自动写入并锁定，Web UI 不允许修改。directory_template 支持变量：{album}、{label}、{authors}、{year}、{date}；选择“平铺”预设可直接保存到输出目录下的专辑文件夹中。",
    "guide.behavior.label": "同步行为 [behavior]",
    "guide.behavior.skipExisting": "skip_existing：跳过已存在目录。",
    "guide.behavior.singleThreaded": "single_threaded：单线程下载，减轻服务器压力。",
    "guide.behavior.maxConcurrent": "max_concurrent_albums：关闭单线程后同时处理的专辑数。",
    "guide.behavior.metadata":
      "generate_readme / generate_nfo：生成媒体库元数据文件。metadata_only：只下载封面、README、NFO，不下载音频。",
    "guide.behavior.debug": "debug：输出更详细的 HTTP 调试日志。",
    "guide.schedule.label": "自动同步 [schedule]",
    "guide.schedule.body":
      "enabled 开启后，Web GUI 模式会按 cron 表达式自动执行全量同步；表达式使用 7 段格式：秒 分 时 日 月 周 年。",
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
    "common.yes": "Yes",
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
    "status.schedule": "Auto sync",
    "status.scheduleEnabled": "Enabled",
    "status.scheduleDisabled": "Disabled",
    "status.scheduleCron": "Cron expression",
    "status.nextRun": "Next run",
    "status.lastRun": "Last run",
    "status.scheduleLastError": "Auto sync error: {message}",
    "status.lastError": "Last error: {message}",
    "status.accountCount": ({ count }) => `${count} account${count === 1 ? "" : "s"}`,
    "sync.title": "Sync controls",
    "sync.info": "Only one sync job can run at a time. Watch the status card after a job starts.",
    "sync.all": "Sync all purchased albums",
    "album.title": "Purchased albums",
    "album.cover": "Cover",
    "album.name": "Title",
    "album.label": "Label",
    "album.actions": "Actions",
    "album.local": "Local state",
    "album.localDownloaded": "Downloaded",
    "album.localNotDownloaded": "Not downloaded",
    "album.localPartial": "Partial",
    "album.localPath": "Local path",
    "album.detail": "Details",
    "album.sync": "Sync",
    "album.reload": "Reload",
    "album.columns": "Columns",
    "album.tableView": "Table",
    "album.cardView": "Cards",
    "album.releaseDate": "Release date",
    "album.tracks": "Tracks",
    "album.trackCount": ({ count }) => `${count} tracks`,
    "album.downloadedTrackCount": ({ count }) => `${count} downloaded`,
    "album.trackProgress": ({ downloaded, total }) => `${downloaded}/${total} tracks`,
    "album.formats": "Formats",
    "album.gift": "Gift",
    "album.id": "ID",
    "detail.title": "Album details",
    "detail.sync": "Sync this album",
    "detail.openInDizzylab": "Open in Dizzylab",
    "detail.openLocalFile": "Open local file",
    "detail.playTrack": "Play / select",
    "detail.selectedTrack": "Current track",
    "detail.releaseDate": "Release date",
    "detail.gift": "Gift",
    "detail.hasGift": "Yes",
    "detail.noGift": "No",
    "detail.tags": "Tags",
    "detail.localSummary": "Local files",
    "detail.localSummaryValue": "{downloaded}/{expected} tracks, {audio} audio files",
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
    "config.testLogin": "Test login",
    "config.testLoginSuccess": "Login test succeeded",
    "config.testLoginFailed": "Login test failed",
    "config.testLoginSuccessUser": "Logged in and fetched user: {username} (UID: {uid})",
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
    "config.template.flat": "Flat: album",
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
    "config.scheduleEnabled": "Enable auto sync",
    "config.scheduleCron": "Auto-sync cron",
    "config.scheduleCronRequired": "Enter a cron expression",
    "config.scheduleCronHelp":
      "Use 7 fields: second minute hour day month weekday year. Example: 0 0 3 * * * * runs daily at 03:00.",
    "config.save": "Save configuration",
    "config.saveOnboarding": "Validate and finish setup",
    "config.saved": "Configuration validated and saved",
    "logs.title": "Log viewer",
    "logs.description":
      "Shows recent Web API and sync job logs for troubleshooting login, configuration, and downloads.",
    "logs.refresh": "Refresh logs",
    "logs.empty": "No logs yet",
    "logs.filterDate": "Filter by date",
    "logs.filterLevel": "Filter by level",
    "logs.filterStart": "Start time",
    "logs.filterEnd": "End time",
    "footer.disclaimer":
      "DizzySync is an unofficial project and is not affiliated with, endorsed by, or partnered with Dizzylab. Please follow Dizzylab terms and local laws, and only sync content you are allowed to access.",
    "footer.source": "Source code",
    "footer.dizzylab": "Dizzylab website",
    "player.title": "Global audio player",
    "player.empty": "Select a downloaded local track from album details",
    "player.play": "Play",
    "player.pause": "Pause",
    "player.previous": "Previous",
    "player.next": "Next",
    "player.loopOff": "Loop off",
    "player.loopOne": "Single-track loop",
    "player.loopAll": "List loop",
    "guide.title": "Configuration guide",
    "guide.user.label": "Login credentials [[users]]",
    "guide.user.body":
      "Configure one or more Dizzylab accounts. username and password are the login credentials. Initial setup and future changes happen in the Web UI. Saving validates every account against Dizzylab before writing config. Legacy [user] config is still read for compatibility.",
    "guide.download.label": "Download formats [download]",
    "guide.download.conflict":
      "128 and 320 both output .mp3 files and cannot be selected together.",
    "guide.paths.label": "Paths and directory template [paths]",
    "guide.paths.body":
      "output_dir is the download directory. When DIZZYSYNC_OUTPUT_DIR is set it is written automatically and locked in the Web UI. directory_template supports {album}, {label}, {authors}, {year}, and {date}; choose the flat preset to save albums directly under the output directory.",
    "guide.behavior.label": "Sync behavior [behavior]",
    "guide.behavior.skipExisting": "skip_existing: skip directories that already exist.",
    "guide.behavior.singleThreaded":
      "single_threaded: download one album at a time to reduce server pressure.",
    "guide.behavior.maxConcurrent":
      "max_concurrent_albums: number of albums processed at once when single-threaded mode is off.",
    "guide.behavior.metadata":
      "generate_readme / generate_nfo: generate media-library metadata. metadata_only: download covers, README, and NFO only, not audio.",
    "guide.behavior.debug": "debug: print more detailed HTTP debug logs.",
    "guide.schedule.label": "Auto sync [schedule]",
    "guide.schedule.body":
      "When enabled is true, Web GUI mode runs a full sync automatically according to the cron expression. The expression uses 7 fields: second minute hour day month weekday year.",
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
