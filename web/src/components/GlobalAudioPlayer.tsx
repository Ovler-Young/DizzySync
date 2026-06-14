import {
  PauseCircleOutlined,
  PlayCircleOutlined,
  RetweetOutlined,
  StepBackwardOutlined,
  StepForwardOutlined,
  SwapOutlined,
  UnorderedListOutlined,
} from "@ant-design/icons";
import { Button, Empty, Image, List, Popover, Space, Tooltip, Typography } from "antd";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { localFileUrl } from "../api.ts";
import { useI18n } from "../i18n.tsx";

export type PlaybackLoopMode = "none" | "one" | "shuffle" | "all";

export interface PlayerTrack {
  key: string;
  albumId: string;
  albumTitle: string;
  trackId: string;
  title: string;
  authors: string;
  path: string;
  cover?: string | null;
}

export interface PlayerSelection {
  albumId: string;
  albumTitle: string;
  tracks: PlayerTrack[];
  index: number;
  requestId: number;
}

interface GlobalAudioPlayerProps {
  selection: PlayerSelection | null;
  onSelectIndex: (index: number) => void;
}

function clampIndex(index: number, length: number) {
  if (length <= 0) {
    return 0;
  }
  return Math.min(Math.max(index, 0), length - 1);
}

export function GlobalAudioPlayer({ selection, onSelectIndex }: GlobalAudioPlayerProps) {
  const { t } = useI18n();
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const previousRequestId = useRef<number | null>(null);
  const shouldPlayAfterSelection = useRef(false);
  const [isPlaying, setIsPlaying] = useState(false);
  const [loopMode, setLoopMode] = useState<PlaybackLoopMode>("none");

  const tracks = selection ? selection.tracks : [];
  const activeIndex = clampIndex(selection ? selection.index : 0, tracks.length);
  const currentTrack = tracks[activeIndex] ?? null;
  const currentSrc = currentTrack ? localFileUrl(currentTrack.path) : undefined;
  const currentCover = currentTrack?.cover ?? null;
  const hasPrevious = tracks.length > 1 && (activeIndex > 0 || loopMode === "all");
  const hasNext =
    tracks.length > 1 &&
    (activeIndex < tracks.length - 1 || loopMode === "all" || loopMode === "shuffle");

  const trackLabel = useMemo(() => {
    if (!currentTrack) {
      return t("player.empty");
    }
    return currentTrack.authors
      ? `${currentTrack.title} — ${currentTrack.authors}`
      : currentTrack.title;
  }, [currentTrack, t]);

  const preservePlaybackForSelection = useCallback(
    (forcePlay = false) => {
      const audio = audioRef.current;
      shouldPlayAfterSelection.current = forcePlay || isPlaying || Boolean(audio && !audio.paused);
    },
    [isPlaying],
  );

  const selectPrevious = useCallback(() => {
    if (tracks.length === 0) {
      return;
    }
    if (activeIndex > 0) {
      preservePlaybackForSelection();
      onSelectIndex(activeIndex - 1);
    } else if (loopMode === "all") {
      preservePlaybackForSelection();
      onSelectIndex(tracks.length - 1);
    }
  }, [activeIndex, loopMode, onSelectIndex, preservePlaybackForSelection, tracks.length]);

  const selectNext = useCallback(
    (forcePlay = false) => {
      if (tracks.length === 0) {
        return;
      }
      if (loopMode === "shuffle" && tracks.length > 1) {
        preservePlaybackForSelection(forcePlay);
        const nextIndex = Math.floor(Math.random() * (tracks.length - 1));
        onSelectIndex(nextIndex >= activeIndex ? nextIndex + 1 : nextIndex);
        return;
      }
      if (activeIndex < tracks.length - 1) {
        preservePlaybackForSelection(forcePlay);
        onSelectIndex(activeIndex + 1);
      } else if (loopMode === "all") {
        preservePlaybackForSelection(forcePlay);
        onSelectIndex(0);
      }
    },
    [activeIndex, loopMode, onSelectIndex, preservePlaybackForSelection, tracks.length],
  );

  const togglePlay = useCallback(() => {
    const audio = audioRef.current;
    if (!(audio && currentTrack)) {
      return;
    }
    if (audio.paused) {
      audio.play().catch(() => undefined);
    } else {
      audio.pause();
    }
  }, [currentTrack]);

  const toggleLoopMode = useCallback(() => {
    setLoopMode((previous) => {
      if (previous === "none") {
        return "one";
      }
      if (previous === "one") {
        return "shuffle";
      }
      if (previous === "shuffle") {
        return "all";
      }
      return "none";
    });
  }, []);

  useEffect(() => {
    const audio = audioRef.current;
    if (!(audio && selection && currentTrack)) {
      setIsPlaying(false);
      return;
    }

    const shouldPlay =
      previousRequestId.current !== selection.requestId || shouldPlayAfterSelection.current;
    previousRequestId.current = selection.requestId;
    shouldPlayAfterSelection.current = false;

    if (shouldPlay) {
      audio.play().catch(() => undefined);
    }
  }, [currentTrack, selection]);

  const selectQueueIndex = useCallback(
    (index: number) => {
      if (index === activeIndex) {
        return;
      }
      preservePlaybackForSelection();
      onSelectIndex(index);
    },
    [activeIndex, onSelectIndex, preservePlaybackForSelection],
  );

  const handleEnded = useCallback(() => {
    if (loopMode === "one") {
      const audio = audioRef.current;
      if (audio) {
        audio.currentTime = 0;
        audio.play().catch(() => undefined);
      }
      return;
    }
    if (activeIndex < tracks.length - 1 || loopMode === "all" || loopMode === "shuffle") {
      selectNext(true);
    } else {
      setIsPlaying(false);
    }
  }, [activeIndex, loopMode, selectNext, tracks.length]);

  let loopLabel = t("player.loopOff");
  if (loopMode === "one") {
    loopLabel = t("player.loopOne");
  } else if (loopMode === "shuffle") {
    loopLabel = t("player.loopShuffle");
  } else if (loopMode === "all") {
    loopLabel = t("player.loopAll");
  }
  const playPauseIcon = (
    <span className="global-player-icon-stack" aria-hidden="true">
      <PlayCircleOutlined
        className={`global-player-icon-layer ${
          isPlaying ? "global-player-icon-layer-inactive" : "global-player-icon-layer-active"
        }`}
      />
      <PauseCircleOutlined
        className={`global-player-icon-layer ${
          isPlaying ? "global-player-icon-layer-active" : "global-player-icon-layer-inactive"
        }`}
      />
    </span>
  );
  const loopIcon = (
    <span className="global-player-icon-stack" aria-hidden="true">
      <RetweetOutlined
        className={`global-player-icon-layer ${
          loopMode === "shuffle"
            ? "global-player-icon-layer-inactive"
            : "global-player-icon-layer-active"
        }`}
      />
      <SwapOutlined
        className={`global-player-icon-layer ${
          loopMode === "shuffle"
            ? "global-player-icon-layer-active"
            : "global-player-icon-layer-inactive"
        }`}
      />
    </span>
  );

  const queueContent = (
    <List
      className="global-player-queue"
      dataSource={tracks}
      locale={{ emptyText: t("player.queueEmpty") }}
      renderItem={(track, index) => {
        const label = track.authors ? `${track.title} — ${track.authors}` : track.title;
        return (
          <List.Item
            className={index === activeIndex ? "global-player-queue-active" : undefined}
            onClick={() => selectQueueIndex(index)}
          >
            <Space align="center" size={8} style={{ minWidth: 0 }}>
              <Typography.Text className="muted">
                {String(index + 1).padStart(2, "0")}
              </Typography.Text>
              <Typography.Text strong={index === activeIndex} ellipsis={{ tooltip: label }}>
                {label}
              </Typography.Text>
            </Space>
          </List.Item>
        );
      }}
      size="small"
    />
  );

  return (
    <section className="global-player" aria-label={t("player.title")}>
      <div className="global-player-info">
        {currentTrack ? (
          <Space align="center" size={12} style={{ minWidth: 0, width: "100%" }}>
            {currentCover ? (
              <Image
                alt={`${currentTrack.albumTitle} cover`}
                className="global-player-cover"
                preview={false}
                referrerPolicy="no-referrer"
                src={currentCover}
              />
            ) : (
              <div className="global-player-cover global-player-cover-placeholder" />
            )}
            <Space direction="vertical" size={0} style={{ minWidth: 0, width: "100%" }}>
              <Typography.Text
                className="global-player-album"
                ellipsis={{ tooltip: currentTrack.albumTitle }}
              >
                {currentTrack.albumTitle}
              </Typography.Text>
              <Typography.Text strong={true} ellipsis={{ tooltip: trackLabel }}>
                {trackLabel}
              </Typography.Text>
            </Space>
          </Space>
        ) : (
          <Empty
            className="global-player-empty"
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("player.empty")}
          />
        )}
      </div>
      <Space className="global-player-controls" size="small" wrap={true}>
        <Tooltip title={t("player.previous")}>
          <Button
            disabled={!hasPrevious}
            icon={<StepBackwardOutlined />}
            onClick={selectPrevious}
          />
        </Tooltip>
        <Tooltip title={isPlaying ? t("player.pause") : t("player.play")}>
          <Button
            disabled={!currentTrack}
            icon={playPauseIcon}
            type="primary"
            onClick={togglePlay}
          />
        </Tooltip>
        <Tooltip title={t("player.next")}>
          <Button disabled={!hasNext} icon={<StepForwardOutlined />} onClick={() => selectNext()} />
        </Tooltip>
        <Tooltip title={loopLabel}>
          <Button
            aria-label={loopLabel}
            className={loopMode === "none" ? undefined : "global-player-loop-active"}
            icon={loopIcon}
            onClick={toggleLoopMode}
          />
        </Tooltip>
        <Popover content={queueContent} placement="top" title={t("player.queue")} trigger="click">
          <Button
            aria-label={t("player.queue")}
            disabled={tracks.length === 0}
            icon={<UnorderedListOutlined />}
          />
        </Popover>
      </Space>
      <audio
        ref={audioRef}
        className="global-player-audio"
        controls={true}
        preload="metadata"
        src={currentSrc}
        onEnded={handleEnded}
        onPause={() => setIsPlaying(false)}
        onPlay={() => setIsPlaying(true)}
      >
        <track kind="captions" />
      </audio>
    </section>
  );
}
