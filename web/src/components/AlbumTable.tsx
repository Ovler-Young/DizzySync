import {
  AppstoreOutlined,
  BarsOutlined,
  EyeOutlined,
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
  onRefresh: () => void;
  onShow: (id: string) => void;
  onSync: (id: string) => void;
}

interface AlbumActionsProps {
  album: DiscListItem;
  syncDisabled: boolean;
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
  return album.local?.path ? <Tooltip title={album.local.path}>{tag}</Tooltip> : tag;
}

function AlbumActions({ album, syncDisabled, onShow, onSync }: AlbumActionsProps) {
  const { t } = useI18n();
  const showAlbum = useCallback(() => onShow(album.id), [album.id, onShow]);
  const syncAlbum = useCallback(() => onSync(album.id), [album.id, onSync]);

  return (
    <Space wrap={true}>
      <Button icon={<EyeOutlined />} onClick={showAlbum}>
        {t("album.detail")}
      </Button>
      {album.local?.downloaded ? null : (
        <Button disabled={syncDisabled} icon={<SyncOutlined />} onClick={syncAlbum}>
          {t("album.sync")}
        </Button>
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
  onRefresh,
  onShow,
  onSync,
}: AlbumTableProps) {
  const { t } = useI18n();
  const [viewMode, setViewMode] = useState<AlbumViewMode>("table");
  const [visibleColumns, setVisibleColumns] = useState<AlbumColumnKey[]>(defaultVisibleColumns);

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
        width: 56,
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
        width: 180,
        render: (_, album) => (
          <AlbumActions album={album} syncDisabled={syncDisabled} onShow={onShow} onSync={onSync} />
        ),
      },
    }),
    [onShow, onSync, syncDisabled, t],
  );

  const columns = useMemo(
    () => visibleColumns.map((key) => allColumns[key]).filter(Boolean),
    [allColumns, visibleColumns],
  );

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

  return (
    <Card title={t("album.title")} extra={toolbar}>
      {viewMode === "table" ? (
        <Table
          columns={columns}
          dataSource={albums}
          loading={loading}
          pagination={{ pageSize: 10, showSizeChanger: true }}
          rowKey="id"
          scroll={{ x: "max-content" }}
        />
      ) : (
        <div className="album-card-grid" aria-busy={loading}>
          {albums.map((album) => (
            <Card
              className="album-card"
              key={album.id}
              hoverable={true}
              cover={<AlbumCover album={album} large={true} />}
              actions={[
                <Button
                  key="detail"
                  type="link"
                  icon={<EyeOutlined />}
                  onClick={() => onShow(album.id)}
                >
                  {t("album.detail")}
                </Button>,
                album.local?.downloaded ? null : (
                  <Button
                    key="sync"
                    disabled={syncDisabled}
                    type="link"
                    icon={<SyncOutlined />}
                    onClick={() => onSync(album.id)}
                  >
                    {t("album.sync")}
                  </Button>
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
