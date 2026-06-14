import { EyeOutlined, ReloadOutlined, SyncOutlined } from "@ant-design/icons";
import { Button, Card, Image, Space, Table, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { useCallback, useMemo } from "react";
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

function AlbumActions({ albumId, syncDisabled, onShow, onSync }: AlbumActionsProps) {
  const showAlbum = useCallback(() => onShow(albumId), [albumId, onShow]);
  const syncAlbum = useCallback(() => onSync(albumId), [albumId, onSync]);

  return (
    <Space>
      <Button icon={<EyeOutlined />} onClick={showAlbum}>
        详情
      </Button>
      <Button disabled={syncDisabled} icon={<SyncOutlined />} onClick={syncAlbum}>
        同步
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
  const columns = useMemo<ColumnsType<DiscListItem>>(
    () => [
      {
        title: "封面",
        dataIndex: "cover",
        width: 72,
        render: (cover: string, album) =>
          cover ? (
            <Image
              alt={`${album.title} cover`}
              className="cover-thumb"
              preview={false}
              src={cover}
            />
          ) : (
            <div className="cover-thumb" />
          ),
      },
      {
        title: "标题",
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
        title: "厂牌",
        dataIndex: "label",
        sorter: (left, right) => left.label.localeCompare(right.label),
      },
      {
        title: "操作",
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
    [onShow, onSync, syncDisabled],
  );

  return (
    <Card
      title="已购专辑"
      extra={
        <Button icon={<ReloadOutlined />} loading={loading} onClick={onRefresh}>
          重新加载
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
