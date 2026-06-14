import { ExportOutlined, SyncOutlined } from "@ant-design/icons";
import { Button, Descriptions, Drawer, Image, List, Space, Tag, Tooltip, Typography } from "antd";
import { useCallback } from "react";
import { localFileUrl } from "../api.ts";
import { useI18n } from "../i18n.tsx";
import type { DiscInfo, Track } from "../types.ts";

interface AlbumDetailDrawerProps {
  album: DiscInfo | null;
  onClose: () => void;
  onSync: (id: string) => void;
}

interface AlbumActionsProps {
  albumId: string;
  onSync: (id: string) => void;
}

function AlbumActions({ albumId, onSync }: AlbumActionsProps) {
  const { t } = useI18n();
  const syncAlbum = useCallback(() => onSync(albumId), [albumId, onSync]);
  const albumUrl = `https://www.dizzylab.net/d/${albumId}/`;

  return (
    <Space>
      <Button href={albumUrl} icon={<ExportOutlined />} rel="noopener noreferrer" target="_blank">
        {t("detail.openInDizzylab")}
      </Button>
      <Button icon={<SyncOutlined />} type="primary" onClick={syncAlbum}>
        {t("detail.sync")}
      </Button>
    </Space>
  );
}

function renderTrackStatus(track: Track, t: (key: string) => string) {
  const { local } = track;
  const downloaded = local ? local.downloaded : false;
  const label = downloaded ? t("album.localDownloaded") : t("album.localNotDownloaded");
  const tag = <Tag color={downloaded ? "success" : "default"}>{label}</Tag>;
  if (!local || local.paths.length === 0) {
    return tag;
  }
  return <Tooltip title={local.paths.join("\n")}>{tag}</Tooltip>;
}

function renderTrackMedia(track: Track, t: (key: string) => string) {
  const path = track.local?.paths[0];
  if (!path) {
    return null;
  }
  const src = localFileUrl(path);
  return (
    <Space className="track-media" direction="vertical" size={4}>
      <audio controls={true} preload="none" src={src}>
        <track kind="captions" />
      </audio>
      <Button href={src} size="small" target="_blank">
        {t("detail.openLocalFile")}
      </Button>
    </Space>
  );
}

function renderTrack(track: Track, index: number, t: (key: string) => string) {
  return (
    <List.Item extra={renderTrackStatus(track, t)}>
      <Space direction="vertical" size={6} style={{ width: "100%" }}>
        <Typography.Text>
          {String(index + 1).padStart(2, "0")}. {track.title}
          {track.authers ? ` — ${track.authers}` : ""}
        </Typography.Text>
        {renderTrackMedia(track, t)}
      </Space>
    </List.Item>
  );
}

export function AlbumDetailDrawer({ album, onClose, onSync }: AlbumDetailDrawerProps) {
  const { t } = useI18n();

  return (
    <Drawer
      destroyOnHidden={true}
      extra={album ? <AlbumActions albumId={album.id} onSync={onSync} /> : null}
      open={Boolean(album)}
      title={album?.title ?? t("detail.title")}
      width={720}
      onClose={onClose}
    >
      {album ? (
        <Space direction="vertical" size="large" style={{ width: "100%" }}>
          {album.cover ? (
            <Image
              alt={`${album.title} cover`}
              referrerPolicy="no-referrer"
              src={album.cover}
              width={180}
            />
          ) : null}
          <Descriptions bordered={true} column={1} size="small">
            <Descriptions.Item label="ID">{album.id}</Descriptions.Item>
            <Descriptions.Item label={t("album.label")}>{album.label}</Descriptions.Item>
            <Descriptions.Item label={t("detail.releaseDate")}>
              {album.release_date ?? "-"}
            </Descriptions.Item>
            <Descriptions.Item label={t("detail.gift")}>
              {album.hasgift ? t("detail.hasGift") : t("detail.noGift")}
            </Descriptions.Item>
            <Descriptions.Item label={t("detail.tags")}>
              {album.tags.map((tag) => (
                <Tag key={tag}>{tag}</Tag>
              ))}
            </Descriptions.Item>
            <Descriptions.Item label={t("detail.localSummary")}>
              <Space direction="vertical" size={0}>
                <Typography.Text>
                  {t("detail.localSummaryValue", {
                    downloaded: album.local?.downloaded_tracks ?? 0,
                    expected: album.local?.expected_tracks ?? album.tracks.length,
                    audio: album.local?.audio_files ?? 0,
                  })}
                </Typography.Text>
                {album.local?.path ? (
                  <Typography.Text className="muted">{album.local.path}</Typography.Text>
                ) : null}
              </Space>
            </Descriptions.Item>
          </Descriptions>
          {album.disc_description ? (
            <Typography.Paragraph>{album.disc_description}</Typography.Paragraph>
          ) : null}
          <List
            bordered={true}
            dataSource={album.tracks}
            header={t("detail.tracks", { count: album.tracks.length })}
            renderItem={(track, index) => renderTrack(track, index, t)}
          />
        </Space>
      ) : null}
    </Drawer>
  );
}
