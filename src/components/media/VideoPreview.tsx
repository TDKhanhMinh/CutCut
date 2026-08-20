import { forwardRef, useImperativeHandle, useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";

interface VideoPreviewProps {
  path: string;
  /** Source-timeline playback clock, kept local to the editor UI. */
  onCurrentTimeChange?: (currentTimeMs: number) => void;
  onDurationChange?: (durationMs: number) => void;
  onPlaybackChange?: (isPlaying: boolean) => void;
}

export type VideoSeekStatus = "applied" | "queued" | "rejected";

export interface VideoSeekResult {
  status: VideoSeekStatus;
  timeMs: number | null;
}

export interface VideoPreviewHandle {
  /** Seek on the source timeline without changing the current play/pause state. */
  seek: (timeMs: number) => VideoSeekResult;
}

export const VideoPreview = forwardRef<VideoPreviewHandle, VideoPreviewProps>(function VideoPreview(
  { path, onCurrentTimeChange, onDurationChange, onPlaybackChange },
  ref,
) {
  // Convert absolute local path to Tauri asset protocol URL
  const assetUrl = convertFileSrc(path);
  const videoRef = useRef<HTMLVideoElement>(null);
  const pendingSeekMsRef = useRef<number | null>(null);

  const applySeek = (video: HTMLVideoElement, requestedTimeMs: number): VideoSeekResult => {
    if (!Number.isFinite(requestedTimeMs) || requestedTimeMs < 0) {
      return { status: "rejected", timeMs: null };
    }

    const durationMs = Number.isFinite(video.duration)
      ? Math.max(0, Math.round(video.duration * 1000))
      : null;
    const targetTimeMs =
      durationMs === null ? Math.round(requestedTimeMs) : Math.min(requestedTimeMs, durationMs);

    try {
      video.currentTime = targetTimeMs / 1000;
      return { status: "applied", timeMs: targetTimeMs };
    } catch {
      return { status: "rejected", timeMs: null };
    }
  };

  useImperativeHandle(
    ref,
    () => ({
      seek: (requestedTimeMs) => {
        if (!Number.isFinite(requestedTimeMs) || requestedTimeMs < 0) {
          return { status: "rejected", timeMs: null };
        }

        const video = videoRef.current;
        if (!video || video.readyState < 1) {
          pendingSeekMsRef.current = Math.round(requestedTimeMs);
          return { status: "queued", timeMs: pendingSeekMsRef.current };
        }

        return applySeek(video, requestedTimeMs);
      },
    }),
    [],
  );

  const handleLoadedMetadata = (event: React.SyntheticEvent<HTMLVideoElement>) => {
    const video = event.currentTarget;
    const pendingSeekMs = pendingSeekMsRef.current;
    if (pendingSeekMs !== null) {
      pendingSeekMsRef.current = null;
      applySeek(video, pendingSeekMs);
    }

    const currentTimeMs = Math.round(video.currentTime * 1000);
    const durationMs = Math.round(video.duration * 1000);
    if (Number.isFinite(currentTimeMs)) onCurrentTimeChange?.(currentTimeMs);
    if (Number.isFinite(durationMs)) onDurationChange?.(durationMs);
  };

  return (
    <div className="m-4 max-w-xl rounded-lg border bg-card p-4 text-card-foreground shadow-sm">
      <h3 className="mb-2 text-lg font-bold">Video Preview</h3>
      <video
        ref={videoRef}
        src={assetUrl}
        controls
        onTimeUpdate={(event) => {
          const currentTimeMs = Math.round(event.currentTarget.currentTime * 1000);
          if (Number.isFinite(currentTimeMs)) onCurrentTimeChange?.(currentTimeMs);
        }}
        onLoadedMetadata={handleLoadedMetadata}
        onPlay={() => onPlaybackChange?.(true)}
        onPause={() => onPlaybackChange?.(false)}
        onEnded={() => onPlaybackChange?.(false)}
        className="w-full rounded bg-black"
        style={{ maxHeight: "400px" }}
      />
    </div>
  );
});

VideoPreview.displayName = "VideoPreview";
