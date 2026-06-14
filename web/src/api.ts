import type {
  ApiMessage,
  ConfigResponse,
  DiscInfo,
  DiscListItem,
  LogEntry,
  StatusResponse,
  UpdateConfigRequest,
} from "./types.ts";

export const apiKeyStorageKey = "dizzysync.apiKey";

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

export const api = {
  status: () => request<StatusResponse>("/api/status"),
  config: () => request<ConfigResponse>("/api/config"),
  logs: () => request<LogEntry[]>("/api/logs"),
  updateConfig: (body: UpdateConfigRequest) =>
    request<ConfigResponse>("/api/config", {
      method: "PUT",
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
