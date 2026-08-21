import { convertFileSrc } from "@tauri-apps/api/core";
import { forwardRef, useCallback, useImperativeHandle, useRef, useState } from "react";
import type { EditPlan, CaptionCue, CaptionStyle } from "@/types/project";
import { useCutPreview } from "@/hooks/useCutPreview";
import { Switch } from "@/components/ui/switch";
import { CaptionOverlay } from "./CaptionOverlay";

export type VideoSeekStatus = "applied" | "queued" | "rejected";

export interface VideoSeekResult {
  status: VideoSeekStatus;
  timeMs: number | null;
}

export interface VideoPreviewRef {
  /** Seek without changing the current play/pause state. */
  seek: (timeMs: number) => VideoSeekResult;
  /** Compatibility alias used by review controls. */
  seekTo: (timeMs: number) => void;
  /** Play a source-time range, with a one-second lead-in. */
  playRange: (startMs: number, endMs: number) => void;
}

export type VideoPreviewHandle = VideoPreviewRef;

interface VideoPreviewProps {
  path: string;
  sourceMediaId?: string;
  editPlan?: EditPlan | null;
  captionCues?: CaptionCue[];
  captionStyle?: CaptionStyle | null;
  onCurrentTimeChange?: (currentTimeMs: number) => void;
  onDurationChange?: (durationMs: number) => void;
  onPlaybackChange?: (isPlaying: boolean) => void;
}

export const VideoPreview = forwardRef<VideoPreviewRef, VideoPreviewProps>(function VideoPreview(
  {
    path,
    sourceMediaId,
    editPlan,
    captionCues,
    captionStyle,
    onCurrentTimeChange,
    onDurationChange,
    onPlaybackChange,
  },
  ref,
) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const pendingSeekMsRef = useRef<number | null>(null);
  const rangeRef = useRef<{ stopMs: number } | null>(null);
  const bypassCutPreviewRef = useRef(false);
  const [cutPreviewActive, setCutPreviewActive] = useState(false);
  const [showCaptions, setShowCaptions] = useState(true);

  const { handleUserSeek } = useCutPreview({
    videoRef,
    plan: editPlan ?? null,
    enabled: cutPreviewActive,
    sourceMediaId,
    bypassRef: bypassCutPreviewRef,
  });

  const assetUrl = convertFileSrc(path);

  const applySeek = useCallback((video: HTMLVideoElement, requestedTimeMs: number) => {
    if (!Number.isFinite(requestedTimeMs) || requestedTimeMs < 0) {
      return { status: "rejected", timeMs: null } satisfies VideoSeekResult;
    }

    const durationMs = Number.isFinite(video.duration)
      ? Math.max(0, Math.round(video.duration * 1000))
      : null;
    const targetTimeMs =
      durationMs === null
        ? Math.round(requestedTimeMs)
        : Math.min(Math.round(requestedTimeMs), durationMs);

    try {
      video.currentTime = targetTimeMs / 1000;
      return { status: "applied", timeMs: targetTimeMs } satisfies VideoSeekResult;
    } catch {
      return { status: "rejected", timeMs: null } satisfies VideoSeekResult;
    }
  }, []);

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

        rangeRef.current = null;
        bypassCutPreviewRef.current = false;
        return applySeek(video, requestedTimeMs);
      },
      seekTo: (requestedTimeMs) => {
        rangeRef.current = null;
        bypassCutPreviewRef.current = false;
        void (
          videoRef.current &&
          (videoRef.current.readyState < 1
            ? (pendingSeekMsRef.current = Math.max(0, Math.round(requestedTimeMs)))
            : applySeek(videoRef.current, requestedTimeMs))
        );
      },
      playRange: (startMs, endMs) => {
        const video = videoRef.current;
        if (!video || !Number.isFinite(startMs) || !Number.isFinite(endMs)) return;
        const boundedStart = Math.max(0, Math.round(startMs));
        const boundedEnd = Math.max(boundedStart, Math.round(endMs));
        const playStartMs = Math.max(0, boundedStart - 1000);
        const contextStopMs = boundedEnd + 1000;
        rangeRef.current = { stopMs: contextStopMs };
        // Suggestion context must expose the cut itself even when cut-preview
        // mode is enabled; restore skipping after the context range completes.
        bypassCutPreviewRef.current = true;
        applySeek(video, playStartMs);
        void video.play().catch(() => undefined);
      },
    }),
    [applySeek],
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

  const handleTimeUpdate = (event: React.SyntheticEvent<HTMLVideoElement>) => {
    const video = event.currentTarget;
    const currentMs = Math.round(video.currentTime * 1000);
    if (Number.isFinite(currentMs)) onCurrentTimeChange?.(currentMs);

    const range = rangeRef.current;
    if (!range) return;
    if (currentMs >= range.stopMs) {
      video.pause();
      rangeRef.current = null;
      bypassCutPreviewRef.current = false;
    }
  };

  const hasCuts = Boolean(
    editPlan?.actions.some((action) => action.enabled && action.type === "cut"),
  );
  const hasCaptions = Boolean(captionCues && captionCues.length > 0);

  return (
    <div className="m-4 max-w-xl rounded-lg border bg-card p-4 text-card-foreground shadow-sm">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-lg font-bold">Video Preview</h3>
        <div className="flex items-center gap-4">
          {hasCaptions && (
            <div className="flex items-center gap-2">
              <span className="text-sm text-muted-foreground">Captions</span>
              <Switch
                id="captions-toggle"
                checked={showCaptions}
                onCheckedChange={setShowCaptions}
                aria-label="Toggle captions"
              />
            </div>
          )}
          {hasCuts && (
            <div className="flex items-center gap-2">
              <span className="text-sm text-muted-foreground">Preview Cuts</span>
              <Switch
                id="cut-preview-toggle"
                checked={cutPreviewActive}
                onCheckedChange={setCutPreviewActive}
                aria-label="Toggle cut-preview mode"
              />
            </div>
          )}
        </div>
      </div>

      {cutPreviewActive && (
        <p className="mb-2 text-xs italic text-muted-foreground">
          Source-time preview — enabled cuts will be skipped.
        </p>
      )}

      <div
        className="@container relative w-full overflow-hidden rounded bg-black"
        style={{ maxHeight: "400px" }}
      >
        <video
          ref={videoRef}
          src={assetUrl}
          controls
          className="h-full w-full object-contain"
          style={{ maxHeight: "400px" }}
          onLoadedMetadata={handleLoadedMetadata}
          onTimeUpdate={handleTimeUpdate}
          onSeeked={handleUserSeek}
          onPlay={() => onPlaybackChange?.(true)}
          onPause={() => onPlaybackChange?.(false)}
          onEnded={() => {
            rangeRef.current = null;
            bypassCutPreviewRef.current = false;
            onPlaybackChange?.(false);
          }}
        />
        {showCaptions && hasCaptions && (
          <CaptionOverlay
            videoRef={videoRef}
            cues={captionCues ?? []}
            styleModel={captionStyle ?? null}
            editPlan={editPlan}
            sourceMediaId={sourceMediaId}
          />
        )}
      </div>
    </div>
  );
});

VideoPreview.displayName = "VideoPreview";
