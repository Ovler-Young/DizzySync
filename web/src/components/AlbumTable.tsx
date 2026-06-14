import { EyeOutlined, ReloadOutlined, SyncOutlined } from "@ant-design/icons";
import { Button, Card, Image, Space, Table, Tag, Tooltip, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { useCallback, useMemo } from "react";
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
  albumId: string;
  syncDisabled: boolean;
  onShow: (id: string) => void;
  onSync: (id: string) => void;
}

function hasPartialLocalState(album: DiscListItem) {
  if (!album.local) {
    return false;
  }
  return album.local.downloaded_tracks > 0 || album.local.audio_files > 0;
}

function localStateColor(album: DiscListItem) {
  if (album.local?.downloaded) {
    return "success";
  }
  if (hasPartialLocalState(album)) {
    return "processing";
  }
  return "default";
}

function localStateLabel(album: DiscListItem, t: (key: string) => string) {
  if (album.local?.downloaded) {
    return t("album.localDownloaded");
  }
  if (hasPartialLocalState(album)) {
    return t("album.localPartial");
  }
  return t("album.localNotDownloaded");
}

function AlbumActions({ albumId, syncDisabled, onShow, onSync }: AlbumActionsProps) {
  const { t } = useI18n();
  const showAlbum = useCallback(() => onShow(albumId), [albumId, onShow]);
  const syncAlbum = useCallback(() => onSync(albumId), [albumId, onSync]);

  return (
    <Space>
      <Button icon={<EyeOutlined />} onClick={showAlbum}>
        {t("album.detail")}
      </Button>
      <Button disabled={syncDisabled} icon={<SyncOutlined />} onClick={syncAlbum}>
        {t("album.sync")}
      </Button>
    </Space>
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
  const columns = useMemo<ColumnsType<DiscListItem>>(
    () => [
      {
        title: t("album.cover"),
        dataIndex: "cover",
        width: 72,
        render: (cover: string, album) =>
          cover ? (
            <Image
              alt={`${album.title} cover`}
              className="cover-thumb"
              preview={false}
              referrerPolicy="no-referrer"
              src={cover}
            />
          ) : (
            <div className="cover-thumb" />
          ),
      },
      {
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
      {
        title: t("album.label"),
        dataIndex: "label",
        sorter: (left, right) => left.label.localeCompare(right.label),
      },
      {
        title: t("album.local"),
        key: "local",
        width: 150,
        render: (_, album) => {
          const tag = <Tag color={localStateColor(album)}>{localStateLabel(album, t)}</Tag>;
          return album.local?.path ? <Tooltip title={album.local.path}>{tag}</Tooltip> : tag;
        },
      },
      {
        title: t("album.actions"),
        key: "actions",
        width: 190,
        render: (_, album) => (
          <AlbumActions
            albumId={album.id}
            syncDisabled={syncDisabled}
            onShow={onShow}
            onSync={onSync}
          />
        ),
      },
    ],
    [onShow, onSync, syncDisabled, t],
  );

  return (
    <Card
      title={t("album.title")}
      extra={
        <Button icon={<ReloadOutlined />} loading={loading} onClick={onRefresh}>
          {t("album.reload")}
        </Button>
      }
    >
      <Table
        columns={columns}
        dataSource={albums}
        loading={loading}
        pagination={{ pageSize: 10, showSizeChanger: true }}
        rowKey="id"
      />
    </Card>
  );
}
