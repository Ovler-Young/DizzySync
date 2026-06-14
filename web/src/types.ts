export interface UserInfo {
  uid: string;
  username: string;
}

export interface StatusResponse {
  status: "ok";
  ready: boolean;
  configured: boolean;
  requires_auth: boolean;
  user: UserInfo | null;
  users: UserInfo[];
  job: JobState;
  schedule: ScheduleState;
  last_error: string | null;
}

export interface ScheduleState {
  enabled: boolean;
  cron: string;
  next_run: number | null;
  last_run: number | null;
  last_error: string | null;
}

export interface LogEntry {
  timestamp: number;
  level: "trace" | "debug" | "info" | "warn" | "error";
  message: string;
}

export type JobState = { state: "idle" } | { state: "running"; kind: string; started_at: number };

export interface LocalAlbumState {
  downloaded: boolean;
  directory_exists: boolean;
  path: string;
  audio_files: number;
  expected_tracks: number;
  downloaded_tracks: number;
  complete_tracks: number;
  has_media: boolean;
  complete: boolean;
  gift_exists: boolean;
  formats: Record<string, boolean>;
  missing_formats: string[];
  missing_tracks: string[];
}

export interface LocalTrackState {
  downloaded: boolean;
  has_media: boolean;
  complete: boolean;
  formats: Record<string, boolean>;
  paths: string[];
  missing_formats: string[];
}

export interface DiscListItem {
  id: string;
  title: string;
  label: string;
  cover: string;
  labelid?: unknown;
  release_date?: string | null;
  price?: unknown;
  hasgift?: boolean;
  ispreselling?: boolean;
  onsell?: boolean;
  onlyhavegift?: boolean;
  tags?: string[];
  track_count?: number;
  formats?: string[];
  local?: LocalAlbumState;
}

export interface Track {
  id: string;
  discid: string;
  title: string;
  album?: string | null;
  authers: string;
  label: string;
  url: string;
  coverurl: string;
  local?: LocalTrackState;
}

export interface DiscInfo extends DiscListItem {
  labelcover?: string | null;
  label_description?: string | null;
  disc_description?: string | null;
  disc_description_2?: string | null;
  release_date?: string | null;
  price?: unknown;
  hasgift: boolean;
  ispreselling: boolean;
  onsell: boolean;
  onlyhavegift: boolean;
  tags: string[];
  tracks: Track[];
  local?: LocalAlbumState;
}

export interface ConfigResponse {
  config_path: string;
  exists: boolean;
  config: PublicConfig;
}

export interface PublicConfig {
  user: PublicUserConfig;
  users: PublicUserConfig[];
  download: PublicDownloadConfig;
  paths: PublicPathsConfig;
  behavior: PublicBehaviorConfig;
  schedule: PublicScheduleConfig;
  api: PublicApiConfig;
}

export interface PublicUserConfig {
  username: string;
  has_password: boolean;
}

export interface PublicDownloadConfig {
  formats: string[];
}

export interface PublicPathsConfig {
  output_dir: string;
  directory_template: string;
  output_dir_locked: boolean;
}

export interface PublicBehaviorConfig {
  skip_existing: boolean;
  single_threaded: boolean;
  max_concurrent_albums: number;
  max_concurrent_albums_locked: boolean;
  generate_readme: boolean;
  generate_nfo: boolean;
  debug: boolean;
  metadata_only: boolean;
}

export interface PublicScheduleConfig {
  enabled: boolean;
  cron: string;
}

export interface PublicApiConfig {
  bind: string;
  has_api_key: boolean;
  web_root: string;
}

export interface UpdateUserConfig {
  username?: string;
  password?: string;
}

export interface TestLoginRequest {
  username: string;
  password?: string;
}

export interface TestLoginResponse {
  success: boolean;
  account_username: string;
  user: UserInfo | null;
  message: string;
}

export interface UpdateConfigRequest {
  user?: UpdateUserConfig;
  users?: UpdateUserConfig[];
  download?: {
    formats?: string[];
  };
  paths?: {
    output_dir?: string;
    directory_template?: string;
  };
  behavior?: Partial<Omit<PublicBehaviorConfig, "max_concurrent_albums_locked">>;
  schedule?: Partial<PublicScheduleConfig>;
  api?: {
    api_key?: string;
  };
}

export interface ApiMessage {
  message: string;
}
