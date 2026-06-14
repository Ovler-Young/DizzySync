import {
  FolderOpenOutlined,
  PauseCircleOutlined,
  PlayCircleOutlined,
  RetweetOutlined,
  StepBackwardOutlined,
  StepForwardOutlined,
} from "@ant-design/icons";
import { Button, Empty, Space, Tooltip, Typography } from "antd";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { localFileUrl } from "../api.ts";
import { useI18n } from "../i18n.tsx";

export type PlaybackLoopMode = "none" | "one" | "all";

export interface PlayerTrack {
  key: string;
  albumId: string;
  albumTitle: string;
  trackId: string;
  title: string;
  authors: string;
  path: string;
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
  const [isPlaying, setIsPlaying] = useState(false);
  const [loopMode, setLoopMode] = useState<PlaybackLoopMode>("none");

  const tracks = selection ? selection.tracks : [];
  const activeIndex = clampIndex(selection ? selection.index : 0, tracks.length);
  const currentTrack = tracks[activeIndex] ?? null;
  const currentSrc = currentTrack ? localFileUrl(currentTrack.path) : undefined;
  const hasPrevious = tracks.length > 1 && (activeIndex > 0 || loopMode === "all");
  const hasNext = tracks.length > 1 && (activeIndex < tracks.length - 1 || loopMode === "all");

  const trackLabel = useMemo(() => {
    if (!currentTrack) {
      return t("player.empty");
    }
    return currentTrack.authors
      ? `${currentTrack.title} — ${currentTrack.authors}`
      : currentTrack.title;
  }, [currentTrack, t]);

  const selectPrevious = useCallback(() => {
    if (tracks.length === 0) {
      return;
    }
    if (activeIndex > 0) {
      onSelectIndex(activeIndex - 1);
    } else if (loopMode === "all") {
      onSelectIndex(tracks.length - 1);
    }
  }, [activeIndex, loopMode, onSelectIndex, tracks.length]);

  const selectNext = useCallback(() => {
    if (tracks.length === 0) {
      return;
    }
    if (activeIndex < tracks.length - 1) {
      onSelectIndex(activeIndex + 1);
    } else if (loopMode === "all") {
      onSelectIndex(0);
    }
  }, [activeIndex, loopMode, onSelectIndex, tracks.length]);

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

    if (previousRequestId.current !== selection.requestId) {
      previousRequestId.current = selection.requestId;
      audio.play().catch(() => undefined);
    }
  }, [currentTrack, selection]);

  const handleEnded = useCallback(() => {
    if (loopMode === "one") {
      const audio = audioRef.current;
      if (audio) {
        audio.currentTime = 0;
        audio.play().catch(() => undefined);
      }
      return;
    }
    if (activeIndex < tracks.length - 1 || loopMode === "all") {
      selectNext();
    } else {
      setIsPlaying(false);
    }
  }, [activeIndex, loopMode, selectNext, tracks.length]);

  let loopLabel = t("player.loopOff");
  if (loopMode === "one") {
    loopLabel = t("player.loopOne");
  } else if (loopMode === "all") {
    loopLabel = t("player.loopAll");
  }

  return (
    <section className="global-player" aria-label={t("player.title")}>
      <div className="global-player-info">
        {currentTrack ? (
          <Space direction="vertical" size={0} style={{ width: "100%" }}>
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
            icon={isPlaying ? <PauseCircleOutlined /> : <PlayCircleOutlined />}
            type="primary"
            onClick={togglePlay}
          />
        </Tooltip>
        <Tooltip title={t("player.next")}>
          <Button disabled={!hasNext} icon={<StepForwardOutlined />} onClick={selectNext} />
        </Tooltip>
        <Tooltip title={loopLabel}>
          <Button
            className={loopMode === "none" ? undefined : "global-player-loop-active"}
            icon={<RetweetOutlined />}
            onClick={toggleLoopMode}
          >
            {loopLabel}
          </Button>
        </Tooltip>
        {currentTrack ? (
          <Button href={currentSrc} icon={<FolderOpenOutlined />} target="_blank">
            {t("detail.openLocalFile")}
          </Button>
        ) : null}
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
