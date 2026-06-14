import {
  AppstoreOutlined,
  BarsOutlined,
  EyeOutlined,
  PlayCircleOutlined,
  ReloadOutlined,
  SettingOutlined,
  SyncOutlined,
} from "@ant-design/icons";
import {
  Button,
  Card,
  Checkbox,
  Dropdown,
  Image,
  Input,
  Segmented,
  Space,
  Table,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import type { ColumnsType } from "antd/es/table";
import { useCallback, useMemo, useState } from "react";
import { useI18n } from "../i18n.tsx";
import type { DiscListItem } from "../types.ts";

interface AlbumTableProps {
  albums: DiscListItem[];
  loading: boolean;
  syncDisabled: boolean;
  onPlay: (id: string) => void;
  onRefresh: () => void;
  onShow: (id: string) => void;
  onSync: (id: string) => void;
}

interface AlbumActionsProps {
  album: DiscListItem;
  syncDisabled: boolean;
  onPlay: (id: string) => void;
  onShow: (id: string) => void;
  onSync: (id: string) => void;
}

type AlbumColumnKey =
  | "cover"
  | "title"
  | "label"
  | "local"
  | "releaseDate"
  | "tracks"
  | "formats"
  | "gift"
  | "id"
  | "actions";

type AlbumViewMode = "table" | "cards";
type LocalStateFilter = "all" | "downloaded" | "partial" | "missing";
type PlayableFilter = "all" | "playable" | "not-playable";
type CheckboxValueType = string | number | boolean;

const defaultVisibleColumns: AlbumColumnKey[] = [
  "cover",
  "title",
  "label",
  "local",
  "tracks",
  "actions",
];

function hasPartialLocalState(album: DiscListItem) {
  if (!album.local) {
    return false;
  }
  return album.local.has_media || album.local.downloaded_tracks > 0 || album.local.audio_files > 0;
}

function hasPlayableLocalAudio(album: DiscListItem) {
  return Boolean(album.local && (album.local.audio_files > 0 || album.local.has_media));
}

function localStateValue(album: DiscListItem) {
  if (album.local?.downloaded) {
    return "downloaded";
  }
  if (hasPartialLocalState(album)) {
    return "partial";
  }
  return "missing";
}

function localStateColor(album: DiscListItem) {
  switch (localStateValue(album)) {
    case "downloaded":
      return "success";
    case "partial":
      return "processing";
    default:
      return "default";
  }
}

function localStateLabel(album: DiscListItem, t: (key: string) => string) {
  switch (localStateValue(album)) {
    case "downloaded":
      return t("album.localDownloaded");
    case "partial":
      return t("album.localPartial");
    default:
      return t("album.localNotDownloaded");
  }
}

function trackCountText(
  album: DiscListItem,
  t: (key: string, params?: Record<string, string | number>) => string,
) {
  const { local, track_count: trackCount } = album;
  const total = trackCount ?? (local ? local.expected_tracks : 0);
  const downloaded = local ? local.downloaded_tracks : 0;
  if (total > 0 && downloaded > 0) {
    return t("album.trackProgress", { downloaded, total });
  }
  if (total > 0) {
    return t("album.trackCount", { count: total });
  }
  if (downloaded > 0) {
    return t("album.downloadedTrackCount", { count: downloaded });
  }
  return "-";
}

function formatList(album: DiscListItem) {
  if (album.formats && album.formats.length > 0) {
    return album.formats.join(", ");
  }
  const localFormats = album.local?.formats
    ? Object.entries(album.local.formats)
        .filter(([, present]) => present)
        .map(([format]) => format)
    : [];
  if (localFormats.length > 0) {
    return localFormats.join(", ");
  }
  const configuredFormats = album.local?.formats ? Object.keys(album.local.formats) : [];
  return configuredFormats.length > 0 ? configuredFormats.join(", ") : "-";
}

function LocalTag({ album }: { album: DiscListItem }) {
  const { t } = useI18n();
  const tag = <Tag color={localStateColor(album)}>{localStateLabel(album, t)}</Tag>;
  if (!album.local) {
    return tag;
  }
  const details = [
    album.local.path,
    album.local.missing_formats.length > 0
      ? `Missing formats: ${album.local.missing_formats.join(", ")}`
      : undefined,
    album.local.missing_tracks.length > 0
      ? `Missing tracks: ${album.local.missing_tracks.join("; ")}`
      : undefined,
  ]
    .filter(Boolean)
    .join("\n");
  return details ? <Tooltip title={details}>{tag}</Tooltip> : tag;
}

function AlbumActions({ album, syncDisabled, onPlay, onShow, onSync }: AlbumActionsProps) {
  const { t } = useI18n();
  const playAlbum = useCallback(() => onPlay(album.id), [album.id, onPlay]);
  const showAlbum = useCallback(() => onShow(album.id), [album.id, onShow]);
  const syncAlbum = useCallback(() => onSync(album.id), [album.id, onSync]);
  const playable = hasPlayableLocalAudio(album);

  return (
    <Space className="album-table-actions" wrap={false} size={4}>
      <Tooltip title={t("album.detail")}>
        <Button aria-label={t("album.detail")} icon={<EyeOutlined />} onClick={showAlbum} />
      </Tooltip>
      <Tooltip title={t("album.play")}>
        <Button
          aria-label={t("album.play")}
          disabled={!playable}
          icon={<PlayCircleOutlined />}
          onClick={playAlbum}
        />
      </Tooltip>
      {album.local?.downloaded ? null : (
        <Tooltip title={t("album.sync")}>
          <Button
            aria-label={t("album.sync")}
            disabled={syncDisabled}
            icon={<SyncOutlined />}
            onClick={syncAlbum}
          />
        </Tooltip>
      )}
    </Space>
  );
}

function AlbumCover({ album, large = false }: { album: DiscListItem; large?: boolean }) {
  return album.cover ? (
    <Image
      alt={`${album.title} cover`}
      className={large ? "cover-card" : "cover-thumb"}
      preview={false}
      referrerPolicy="no-referrer"
      src={album.cover}
    />
  ) : (
    <div className={large ? "cover-card cover-placeholder" : "cover-thumb cover-placeholder"} />
  );
}

export function AlbumTable({
  albums,
  loading,
  syncDisabled,
  onPlay,
  onRefresh,
  onShow,
  onSync,
}: AlbumTableProps) {
  const { t } = useI18n();
  const [viewMode, setViewMode] = useState<AlbumViewMode>("table");
  const [visibleColumns, setVisibleColumns] = useState<AlbumColumnKey[]>(defaultVisibleColumns);
  const [localFilter, setLocalFilter] = useState<LocalStateFilter>("all");
  const [playableFilter, setPlayableFilter] = useState<PlayableFilter>("all");
  const [textFilter, setTextFilter] = useState("");

  const columnOptions = useMemo(
    () => [
      { label: t("album.cover"), value: "cover" },
      { label: t("album.name"), value: "title" },
      { label: t("album.label"), value: "label" },
      { label: t("album.local"), value: "local" },
      { label: t("album.releaseDate"), value: "releaseDate" },
      { label: t("album.tracks"), value: "tracks" },
      { label: t("album.formats"), value: "formats" },
      { label: t("album.gift"), value: "gift" },
      { label: t("album.id"), value: "id" },
      { label: t("album.actions"), value: "actions" },
    ],
    [t],
  );

  const allColumns = useMemo<Record<AlbumColumnKey, ColumnsType<DiscListItem>[number]>>(
    () => ({
      cover: {
        title: t("album.cover"),
        className: "album-cover-cell",
        dataIndex: "cover",
        width: 88,
        render: (_, album) => <AlbumCover album={album} />,
      },
      title: {
        title: t("album.name"),
        dataIndex: "title",
        sorter: (left, right) => left.title.localeCompare(right.title),
        render: (title: string, album) => (
          <Space direction="vertical" size={0}>
            <Typography.Text strong={true}>{title}</Typography.Text>
            <Typography.Text className="muted">{album.id}</Typography.Text>
          </Space>
        ),
      },
      label: {
        title: t("album.label"),
        dataIndex: "label",
        sorter: (left, right) => left.label.localeCompare(right.label),
      },
      local: {
        title: t("album.local"),
        key: "local",
        width: 140,
        filters: [
          { text: t("album.localDownloaded"), value: "downloaded" },
          { text: t("album.localPartial"), value: "partial" },
          { text: t("album.localNotDownloaded"), value: "missing" },
        ],
        onFilter: (value, album) => localStateValue(album) === value,
        render: (_, album) => <LocalTag album={album} />,
      },
      releaseDate: {
        title: t("album.releaseDate"),
        dataIndex: "release_date",
        sorter: (left, right) => (left.release_date ?? "").localeCompare(right.release_date ?? ""),
        render: (value?: string | null) => value || "-",
      },
      tracks: {
        title: t("album.tracks"),
        key: "tracks",
        sorter: (left, right) =>
          (left.track_count ?? left.local?.expected_tracks ?? 0) -
          (right.track_count ?? right.local?.expected_tracks ?? 0),
        render: (_, album) => trackCountText(album, t),
      },
      formats: {
        title: t("album.formats"),
        key: "formats",
        render: (_, album) => formatList(album),
      },
      gift: {
        title: t("album.gift"),
        key: "gift",
        render: (_, album) => (album.hasgift || album.local?.gift_exists ? t("common.yes") : "-"),
      },
      id: {
        title: t("album.id"),
        dataIndex: "id",
        render: (id: string) => <Typography.Text copyable={true}>{id}</Typography.Text>,
      },
      actions: {
        title: t("album.actions"),
        key: "actions",
        className: "album-actions-cell",
        width: 168,
        render: (_, album) => (
          <AlbumActions
            album={album}
            syncDisabled={syncDisabled}
            onPlay={onPlay}
            onShow={onShow}
            onSync={onSync}
          />
        ),
      },
    }),
    [onPlay, onShow, onSync, syncDisabled, t],
  );

  const columns = useMemo(
    () => visibleColumns.map((key) => allColumns[key]).filter(Boolean),
    [allColumns, visibleColumns],
  );

  const filteredAlbums = useMemo(() => {
    const normalizedTextFilter = textFilter.trim().toLowerCase();
    return albums.filter((album) => {
      if (localFilter !== "all" && localStateValue(album) !== localFilter) {
        return false;
      }
      if (playableFilter === "playable" && !hasPlayableLocalAudio(album)) {
        return false;
      }
      if (playableFilter === "not-playable" && hasPlayableLocalAudio(album)) {
        return false;
      }
      if (normalizedTextFilter) {
        const searchable = [album.title, album.label, album.id, ...(album.tags ?? [])]
          .join(" ")
          .toLowerCase();
        return searchable.includes(normalizedTextFilter);
      }
      return true;
    });
  }, [albums, localFilter, playableFilter, textFilter]);

  const handleColumnsChange = useCallback((values: CheckboxValueType[]) => {
    const selected = values as AlbumColumnKey[];
    setVisibleColumns(selected.length > 0 ? selected : ["title"]);
  }, []);

  const toolbar = (
    <Space wrap={true}>
      <Segmented
        onChange={(value) => setViewMode(value as AlbumViewMode)}
        options={[
          { label: t("album.tableView"), value: "table", icon: <BarsOutlined /> },
          { label: t("album.cardView"), value: "cards", icon: <AppstoreOutlined /> },
        ]}
        value={viewMode}
      />
      {viewMode === "table" ? (
        <Dropdown
          dropdownRender={() => (
            <Card className="album-column-menu" size="small">
              <Checkbox.Group
                className="album-column-checkboxes"
                options={columnOptions}
                value={visibleColumns}
                onChange={handleColumnsChange}
              />
            </Card>
          )}
          trigger={["click"]}
        >
          <Button icon={<SettingOutlined />}>{t("album.columns")}</Button>
        </Dropdown>
      ) : null}
      <Button icon={<ReloadOutlined />} loading={loading} onClick={onRefresh}>
        {t("album.reload")}
      </Button>
    </Space>
  );

  const quickFilters = (
    <Space className="album-quick-filters" wrap={true}>
      <Segmented
        onChange={(value) => setLocalFilter(value as LocalStateFilter)}
        options={[
          { label: t("album.filterAll"), value: "all" },
          { label: t("album.localDownloaded"), value: "downloaded" },
          { label: t("album.localPartial"), value: "partial" },
          { label: t("album.localNotDownloaded"), value: "missing" },
        ]}
        value={localFilter}
      />
      <Segmented
        onChange={(value) => setPlayableFilter(value as PlayableFilter)}
        options={[
          { label: t("album.filterAllPlayable"), value: "all" },
          { label: t("album.filterPlayable"), value: "playable" },
          { label: t("album.filterNotPlayable"), value: "not-playable" },
        ]}
        value={playableFilter}
      />
      <Input.Search
        allowClear={true}
        className="album-text-filter"
        placeholder={t("album.filterSearchPlaceholder")}
        value={textFilter}
        onChange={(event) => setTextFilter(event.target.value)}
      />
    </Space>
  );

  return (
    <Card title={t("album.title")} extra={toolbar}>
      {quickFilters}
      {viewMode === "table" ? (
        <Table
          columns={columns}
          dataSource={filteredAlbums}
          loading={loading}
          pagination={{ pageSize: 10, showSizeChanger: true }}
          rowKey="id"
          scroll={{ x: "max-content" }}
        />
      ) : (
        <div className="album-card-grid" aria-busy={loading}>
          {filteredAlbums.map((album) => (
            <Card
              className="album-card"
              key={album.id}
              hoverable={true}
              cover={<AlbumCover album={album} large={true} />}
              actions={[
                <Tooltip key="detail" title={t("album.detail")}>
                  <Button
                    type="link"
                    icon={<EyeOutlined />}
                    aria-label={t("album.detail")}
                    onClick={() => onShow(album.id)}
                  />
                </Tooltip>,
                <Tooltip key="play" title={t("album.play")}>
                  <Button
                    aria-label={t("album.play")}
                    disabled={!hasPlayableLocalAudio(album)}
                    type="link"
                    icon={<PlayCircleOutlined />}
                    onClick={() => onPlay(album.id)}
                  />
                </Tooltip>,
                album.local?.downloaded ? null : (
                  <Tooltip key="sync" title={t("album.sync")}>
                    <Button
                      aria-label={t("album.sync")}
                      disabled={syncDisabled}
                      type="link"
                      icon={<SyncOutlined />}
                      onClick={() => onSync(album.id)}
                    />
                  </Tooltip>
                ),
              ].filter(Boolean)}
            >
              <Space direction="vertical" size={6} style={{ width: "100%" }}>
                <Typography.Text strong={true} ellipsis={{ tooltip: album.title }}>
                  {album.title}
                </Typography.Text>
                <Typography.Text className="muted" ellipsis={{ tooltip: album.label }}>
                  {album.label}
                </Typography.Text>
                <Space wrap={true} size={4}>
                  <LocalTag album={album} />
                  <Tag>{trackCountText(album, t)}</Tag>
                  {album.release_date ? <Tag>{album.release_date}</Tag> : null}
                </Space>
              </Space>
            </Card>
          ))}
        </div>
      )}
    </Card>
  );
}
