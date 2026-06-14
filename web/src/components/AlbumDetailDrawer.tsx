import { SyncOutlined } from "@ant-design/icons";
import { Button, Descriptions, Drawer, Image, List, Space, Tag, Typography } from "antd";
import { useCallback } from "react";
import type { DiscInfo, Track } from "../types.ts";

interface AlbumDetailDrawerProps {
  album: DiscInfo | null;
  onClose: () => void;
  onSync: (id: string) => void;
}

interface SyncAlbumButtonProps {
  albumId: string;
  onSync: (id: string) => void;
}

function SyncAlbumButton({ albumId, onSync }: SyncAlbumButtonProps) {
  const syncAlbum = useCallback(() => onSync(albumId), [albumId, onSync]);

  return (
    <Button icon={<SyncOutlined />} type="primary" onClick={syncAlbum}>
      同步此专辑
    </Button>
  );
}

function renderTrack(track: Track, index: number) {
  return (
    <List.Item>
      <Typography.Text>
        {String(index + 1).padStart(2, "0")}. {track.title}
        {track.authers ? ` — ${track.authers}` : ""}
      </Typography.Text>
    </List.Item>
  );
}

export function AlbumDetailDrawer({ album, onClose, onSync }: AlbumDetailDrawerProps) {
  return (
    <Drawer
      destroyOnHidden={true}
      extra={album ? <SyncAlbumButton albumId={album.id} onSync={onSync} /> : null}
      open={Boolean(album)}
      title={album?.title ?? "专辑详情"}
      width={720}
      onClose={onClose}
    >
      {album ? (
        <Space direction="vertical" size="large" style={{ width: "100%" }}>
          {album.cover ? (
            <Image alt={`${album.title} cover`} src={album.cover} width={180} />
          ) : null}
          <Descriptions bordered={true} column={1} size="small">
            <Descriptions.Item label="ID">{album.id}</Descriptions.Item>
            <Descriptions.Item label="厂牌">{album.label}</Descriptions.Item>
            <Descriptions.Item label="发布日期">{album.release_date ?? "-"}</Descriptions.Item>
            <Descriptions.Item label="特典">{album.hasgift ? "有" : "无"}</Descriptions.Item>
            <Descriptions.Item label="标签">
              {album.tags.map((tag) => (
                <Tag key={tag}>{tag}</Tag>
              ))}
            </Descriptions.Item>
          </Descriptions>
          {album.disc_description ? (
            <Typography.Paragraph>{album.disc_description}</Typography.Paragraph>
          ) : null}
          <List
            bordered={true}
            dataSource={album.tracks}
            header={`曲目 (${album.tracks.length})`}
            renderItem={renderTrack}
          />
        </Space>
      ) : null}
    </Drawer>
  );
}
