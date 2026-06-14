import { ExportOutlined, PlayCircleOutlined, SyncOutlined } from "@ant-design/icons";
import { Button, Descriptions, Drawer, Image, List, Space, Tag, Tooltip, Typography } from "antd";
import { useCallback } from "react";
import { useI18n } from "../i18n.tsx";
import type { DiscInfo, Track } from "../types.ts";

interface AlbumDetailDrawerProps {
  album: DiscInfo | null;
  currentTrackKey?: string | null;
  onClose: () => void;
  onPlayTrack: (album: DiscInfo, track: Track) => void;
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
      <Tooltip title={t("detail.openInDizzylab")}>
        <Button
          aria-label={t("detail.openInDizzylab")}
          href={albumUrl}
          icon={<ExportOutlined />}
          rel="noopener noreferrer"
          target="_blank"
        />
      </Tooltip>
      <Tooltip title={t("detail.sync")}>
        <Button
          aria-label={t("detail.sync")}
          icon={<SyncOutlined />}
          type="primary"
          onClick={syncAlbum}
        />
      </Tooltip>
    </Space>
  );
}

function renderTrackStatus(track: Track, t: (key: string) => string) {
  const { local } = track;
  const downloaded = local ? local.downloaded : false;
  const hasMedia = local ? local.has_media || local.paths.length > 0 : false;
  let label = t("album.localNotDownloaded");
  let color = "default";

  if (downloaded) {
    label = t("album.localDownloaded");
    color = "success";
  } else if (hasMedia) {
    label = t("album.localPartial");
    color = "processing";
  }

  const tag = <Tag color={color}>{label}</Tag>;
  if (!local) {
    return tag;
  }
  const details = [
    ...local.paths,
    local.missing_formats.length > 0
      ? `Missing formats: ${local.missing_formats.join(", ")}`
      : undefined,
  ]
    .filter(Boolean)
    .join("\n");
  return details ? <Tooltip title={details}>{tag}</Tooltip> : tag;
}

function trackKey(track: Track) {
  return `${track.discid}:${track.id}`;
}

interface TrackActionsProps {
  album: DiscInfo;
  currentTrackKey?: string | null;
  track: Track;
  onPlayTrack: (album: DiscInfo, track: Track) => void;
}

function TrackActions({ album, currentTrackKey, track, onPlayTrack }: TrackActionsProps) {
  const { t } = useI18n();
  const playable = Boolean(track.local?.paths[0]);
  const isCurrent = currentTrackKey === trackKey(track);
  const playTrack = useCallback(() => onPlayTrack(album, track), [album, onPlayTrack, track]);

  return (
    <Space className="track-actions" wrap={true}>
      <Tooltip title={isCurrent ? t("detail.selectedTrack") : t("detail.playTrack")}>
        <Button
          aria-label={isCurrent ? t("detail.selectedTrack") : t("detail.playTrack")}
          disabled={!playable}
          icon={<PlayCircleOutlined />}
          size="small"
          type={isCurrent ? "primary" : "default"}
          onClick={playTrack}
        />
      </Tooltip>
    </Space>
  );
}

interface RenderTrackOptions {
  album: DiscInfo;
  currentTrackKey?: string | null;
  index: number;
  onPlayTrack: (album: DiscInfo, track: Track) => void;
  t: (key: string) => string;
  track: Track;
}

function renderTrack({ album, currentTrackKey, index, onPlayTrack, t, track }: RenderTrackOptions) {
  return (
    <List.Item extra={renderTrackStatus(track, t)}>
      <Space direction="vertical" size={6} style={{ width: "100%" }}>
        <Typography.Text>
          {String(index + 1).padStart(2, "0")}. {track.title}
          {track.authers ? ` — ${track.authers}` : ""}
        </Typography.Text>
        <TrackActions
          album={album}
          currentTrackKey={currentTrackKey}
          track={track}
          onPlayTrack={onPlayTrack}
        />
      </Space>
    </List.Item>
  );
}

export function AlbumDetailDrawer({
  album,
  currentTrackKey,
  onClose,
  onPlayTrack,
  onSync,
}: AlbumDetailDrawerProps) {
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
              className="detail-cover"
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
                {album.local && album.local.missing_formats.length > 0 ? (
                  <Typography.Text className="muted">
                    Missing formats: {album.local.missing_formats.join(", ")}
                  </Typography.Text>
                ) : null}
                {album.local && album.local.missing_tracks.length > 0 ? (
                  <Typography.Text className="muted">
                    Missing tracks: {album.local.missing_tracks.join("; ")}
                  </Typography.Text>
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
            renderItem={(track, index) =>
              renderTrack({ album, currentTrackKey, index, onPlayTrack, t, track })
            }
          />
        </Space>
      ) : null}
    </Drawer>
  );
}
