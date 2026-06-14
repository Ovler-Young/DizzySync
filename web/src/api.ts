import type {
  ApiMessage,
  ConfigResponse,
  DiscInfo,
  DiscListItem,
  LogEntry,
  StatusResponse,
  TestLoginRequest,
  TestLoginResponse,
  UpdateConfigRequest,
} from "./types.ts";

export const apiKeyStorageKey = "dizzysync.apiKey";

export function localFileUrl(path: string): string {
  const params = new URLSearchParams({ path });
  const apiKey = localStorage.getItem(apiKeyStorageKey)?.trim();
  if (apiKey) {
    params.set("api_key", apiKey);
  }
  return `/api/local-file?${params.toString()}`;
}

export class ApiError extends Error {
  public readonly status: number;

  public constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  const apiKey = localStorage.getItem(apiKeyStorageKey)?.trim();

  if (apiKey) {
    headers.set("X-API-Key", apiKey);
  }
  if (init.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }

  const response = await fetch(path, { ...init, headers });
  const contentType = response.headers.get("content-type") ?? "";
  const text = await response.text();
  let payload: unknown = text;

  if (contentType.includes("application/json") && text) {
    try {
      payload = JSON.parse(text) as unknown;
    } catch {
      payload = text;
    }
  }

  if (!response.ok) {
    const message =
      typeof payload === "object" && payload !== null && "message" in payload
        ? String((payload as ApiMessage).message)
        : text || response.statusText;
    throw new ApiError(response.status, message);
  }

  return payload as T;
}

export interface LogFilters {
  date?: string;
  level?: string;
  start?: string;
  end?: string;
}

function queryString(filters: LogFilters): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filters)) {
    if (value) {
      params.set(key, value);
    }
  }
  const query = params.toString();
  return query ? `?${query}` : "";
}

export const api = {
  status: () => request<StatusResponse>("/api/status"),
  config: () => request<ConfigResponse>("/api/config"),
  logs: (filters: LogFilters = {}) => request<LogEntry[]>(`/api/logs${queryString(filters)}`),
  updateConfig: (body: UpdateConfigRequest) =>
    request<ConfigResponse>("/api/config", {
      method: "PUT",
      body: JSON.stringify(body),
    }),
  testLogin: (body: TestLoginRequest) =>
    request<TestLoginResponse>("/api/config/test-login", {
      method: "POST",
      body: JSON.stringify(body),
    }),
  albums: () => request<DiscListItem[]>("/api/albums"),
  album: (id: string) => request<DiscInfo>(`/api/albums/${encodeURIComponent(id)}`),
  syncAll: () =>
    request<ApiMessage>("/api/sync", {
      method: "POST",
      body: JSON.stringify({}),
    }),
  syncAlbum: (id: string) =>
    request<ApiMessage>(`/api/sync/${encodeURIComponent(id)}`, {
      method: "POST",
    }),
};
