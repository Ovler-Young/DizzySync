import { ExportOutlined, PlayCircleOutlined, SyncOutlined } from "@ant-design/icons";
import { Button, Descriptions, Drawer, Image, Space, Table, Tag, Tooltip, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { useCallback, useMemo } from "react";
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

const numericDurationPattern = /^\d+(?:\.\d+)?$/;

function trackHasPlayableAudio(track: Track) {
  return Boolean(track.local?.paths[0]);
}

function trackStatusRank(track: Track) {
  const { local } = track;
  if (local?.downloaded) {
    return 2;
  }
  if (local?.has_media || (local && local.paths.length > 0)) {
    return 1;
  }
  return 0;
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
      ? `Missing audio formats: ${local.missing_formats.join(", ")}`
      : undefined,
  ]
    .filter(Boolean)
    .join("\n");
  return details ? <Tooltip title={details}>{tag}</Tooltip> : tag;
}

function trackKey(track: Track) {
  return `${track.discid}:${track.id}`;
}

function durationValue(track: Track) {
  return (
    track.duration ??
    track.duration_seconds ??
    track.durationSeconds ??
    track.length ??
    track.length_seconds ??
    track.lengthSeconds ??
    track.time ??
    null
  );
}

function parseDurationSeconds(value: string | number | null | undefined) {
  if (value === null || value === undefined) {
    return null;
  }
  if (typeof value === "number") {
    return Number.isFinite(value) && value > 0 ? Math.round(value) : null;
  }
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  if (numericDurationPattern.test(trimmed)) {
    return Math.round(Number(trimmed));
  }
  const parts = trimmed.split(":").map((part) => Number(part));
  if (parts.length >= 2 && parts.length <= 3 && parts.every(Number.isFinite)) {
    return parts.reduce((total, part) => total * 60 + part, 0);
  }
  return null;
}

function formatDuration(track: Track) {
  const seconds = parseDurationSeconds(durationValue(track));
  if (!seconds) {
    return "-";
  }
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const remainingSeconds = seconds % 60;
  const two = (value: number) => String(value).padStart(2, "0");
  return hours > 0
    ? `${hours}:${two(minutes)}:${two(remainingSeconds)}`
    : `${minutes}:${two(remainingSeconds)}`;
}

interface TrackActionsProps {
  album: DiscInfo;
  currentTrackKey?: string | null;
  track: Track;
  onPlayTrack: (album: DiscInfo, track: Track) => void;
}

function TrackActions({ album, currentTrackKey, track, onPlayTrack }: TrackActionsProps) {
  const { t } = useI18n();
  const playable = trackHasPlayableAudio(track);
  const isCurrent = currentTrackKey === trackKey(track);
  const playTrack = useCallback(() => onPlayTrack(album, track), [album, onPlayTrack, track]);

  return (
    <Space className="track-actions" wrap={false}>
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

interface TrackRow {
  index: number;
  track: Track;
}

export function AlbumDetailDrawer({
  album,
  currentTrackKey,
  onClose,
  onPlayTrack,
  onSync,
}: AlbumDetailDrawerProps) {
  const { t } = useI18n();

  const trackRows = useMemo<TrackRow[]>(
    () => album?.tracks.map((track, index) => ({ index, track })) ?? [],
    [album?.tracks],
  );

  const trackColumns = useMemo<ColumnsType<TrackRow>>(
    () => [
      {
        title: "#",
        dataIndex: "index",
        width: 64,
        sorter: (left, right) => left.index - right.index,
        render: (index: number) => String(index + 1).padStart(2, "0"),
      },
      {
        title: t("album.name"),
        key: "title",
        sorter: (left, right) => left.track.title.localeCompare(right.track.title),
        render: (_, row) => (
          <Space className="track-title-cell" size={8} wrap={false}>
            {album ? (
              <TrackActions
                album={album}
                currentTrackKey={currentTrackKey}
                track={row.track}
                onPlayTrack={onPlayTrack}
              />
            ) : null}
            <Typography.Text className="track-title-text" ellipsis={{ tooltip: row.track.title }}>
              {row.track.title}
            </Typography.Text>
            {row.track.authers ? (
              <Typography.Text
                className="track-author-text"
                ellipsis={{ tooltip: row.track.authers }}
              >
                — {row.track.authers}
              </Typography.Text>
            ) : null}
          </Space>
        ),
      },
      {
        title: t("detail.duration"),
        key: "duration",
        width: 96,
        sorter: (left, right) =>
          (parseDurationSeconds(durationValue(left.track)) ?? 0) -
          (parseDurationSeconds(durationValue(right.track)) ?? 0),
        render: (_, row) => (
          <Typography.Text className="track-duration">{formatDuration(row.track)}</Typography.Text>
        ),
      },
      {
        title: t("album.local"),
        key: "status",
        width: 132,
        sorter: (left, right) => trackStatusRank(left.track) - trackStatusRank(right.track),
        render: (_, row) => renderTrackStatus(row.track, t),
      },
    ],
    [album, currentTrackKey, onPlayTrack, t],
  );

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
              {album.hasgift || album.local?.gift_exists ? t("detail.hasGift") : t("detail.noGift")}
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
                    Missing audio formats: {album.local.missing_formats.join(", ")}
                  </Typography.Text>
                ) : null}
                {album.local?.gift_missing ? (
                  <Typography.Text className="muted">{t("detail.giftMissing")}</Typography.Text>
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
          <Table
            bordered={true}
            className="track-table"
            columns={trackColumns}
            dataSource={trackRows}
            pagination={false}
            rowKey={(row) => trackKey(row.track)}
            size="small"
            title={() => t("detail.tracks", { count: album.tracks.length })}
          />
        </Space>
      ) : null}
    </Drawer>
  );
}
