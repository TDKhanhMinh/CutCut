import { useState, useEffect, useRef } from "react";
import { TranscriptPanel } from "../editor/transcript/TranscriptPanel";
import { Transcript } from "@/types/transcript";
import { useTranscriptSync } from "@/hooks/useTranscriptSync";
import { useProjectStore } from "@/stores/useProjectStore";
import { applyTranscriptTextEdit, revertTranscriptTextEdit } from "@/lib/transcript-edit";
import { VideoPreview, type VideoPreviewHandle } from "@/components/media/VideoPreview";
import type { MediaJobEvent } from "@/services/media";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SilenceDetectionResult } from "@/types/silence";

const mockTranscript: Transcript = {
  id: "t1",
  sourceId: "s1",
  modelId: "m1",
  language: "vi",
  generatedAt: 0,
  segments: Array.from({ length: 1000 }).map((_, i) => ({
    id: `seg-${i}`,
    startMs: i * 2000,
    endMs: i * 2000 + 1900,
    text: `Đây là câu thoại tiếng Việt số ${i + 1} được ảo hoá mượt mà bởi Tanstack Virtual.`,
    speaker: null,
    isFiller: false,
  })),
};

export function Workspace() {
  const [currentTime, setCurrentTime] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [mockTranscriptState, setMockTranscriptState] = useState(mockTranscript);
  const videoPreviewRef = useRef<VideoPreviewHandle>(null);
  const activeProject = useProjectStore((state) => state.activeProject);
  const missingMediaIds = useProjectStore((state) => state.missingMediaIds);
  const updateTranscriptSegmentText = useProjectStore((state) => state.updateTranscriptSegmentText);
  const revertTranscriptSegmentText = useProjectStore((state) => state.revertTranscriptSegmentText);
  const transcript = activeProject ? activeProject.transcript : mockTranscriptState;
  const source = activeProject?.media[0] ?? null;
  const isSourceMissing = source ? missingMediaIds.includes(source.id) : false;

  const activeSegmentId = useTranscriptSync(transcript?.segments ?? [], currentTime);

  const handleTranscriptEdit = (id: string, newText: string) => {
    if (activeProject) {
      updateTranscriptSegmentText(id, newText);
      return;
    }

    setMockTranscriptState((previous) => {
      const segment = previous.segments.find((candidate) => candidate.id === id);
      if (!segment) return previous;
      const result = applyTranscriptTextEdit(segment, newText);
      if (!result.changed) return previous;
      return {
        ...previous,
        segments: previous.segments.map((candidate) =>
          candidate.id === id ? result.segment : candidate,
        ),
      };
    });
  };

  const handleTranscriptRevert = (id: string) => {
    if (activeProject) {
      revertTranscriptSegmentText(id);
      return;
    }

    setMockTranscriptState((previous) => ({
      ...previous,
      segments: previous.segments.map((segment) =>
        segment.id === id ? revertTranscriptTextEdit(segment) : segment,
      ),
    }));
  };

  useEffect(() => {
    if (activeProject && source && !isSourceMissing) return;
    if (!isPlaying) return;
    const interval = setInterval(() => {
      setCurrentTime((prev) => prev + 100);
    }, 100);
    return () => clearInterval(interval);
  }, [activeProject, isPlaying, isSourceMissing, source]);

  const handleDetectSilence = async () => {
    let stopListening: (() => void) | null = null;
    try {
      const jobId = `silence-${Date.now()}`;

      const unlisten = await listen("media-job", (event: { payload: MediaJobEvent }) => {
        const payload = event.payload;
        if (payload.jobId !== jobId) return;

        if (payload.state === "completed") {
          const result = payload.result as SilenceDetectionResult | undefined;
          const intervals = result?.intervals ?? [];
          console.log("Detected silence intervals:", result);
          alert(`Found ${intervals.length} silence intervals! Check console.`);
          unlisten();
        } else if (payload.state === "failed" || payload.state === "cancelled") {
          console.error("Silence detection failed/cancelled:", payload.error || payload.message);
          alert("Silence detection failed: " + (payload.error || payload.message));
          unlisten();
        }
      });
      stopListening = unlisten;

      const detectionPath = source?.path ?? "C:\\non_existent.mp4";
      const durationMs = source?.metadata.durationSec
        ? Math.round(source.metadata.durationSec * 1000)
        : null;

      await invoke("start_silence_detection", {
        jobId,
        path: detectionPath,
        settings: {
          thresholdDb: -35,
          minDurationMs: 500,
          paddingMs: 0,
        },
        durationMs,
      });
    } catch (e) {
      stopListening?.();
      console.error("IPC Error:", e);
      alert("Silence detection could not start: " + (e instanceof Error ? e.message : String(e)));
    }
  };

  return (
    <div className="flex h-full w-full overflow-hidden bg-background">
      <div className="flex flex-1 flex-col items-center justify-center bg-muted/10 p-4">
        {source && !isSourceMissing ? (
          <VideoPreview
            ref={videoPreviewRef}
            path={source.path}
            onCurrentTimeChange={setCurrentTime}
            onPlaybackChange={setIsPlaying}
          />
        ) : (
          <div className="max-w-md space-y-4 rounded-xl border border-dashed border-border bg-background p-8 text-center shadow-sm">
            <h3 className="text-lg font-medium">Video Preview Area</h3>
            <p className="text-sm text-muted-foreground">
              Current simulated time: {(currentTime / 1000).toFixed(1)}s
            </p>
            <div className="flex flex-wrap items-center justify-center gap-2 pt-4">
              <button
                className="cursor-pointer rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
                onClick={() => setIsPlaying(!isPlaying)}
              >
                {isPlaying ? "Pause Playback" : "Start Playback"}
              </button>
              <button
                className="cursor-pointer rounded-md bg-secondary px-4 py-2 text-sm text-secondary-foreground hover:bg-secondary/90"
                onClick={() => setCurrentTime((prev) => prev + 10000)}
              >
                +10s Seek
              </button>
              <button
                className="bg-outline cursor-pointer rounded-md border border-border px-4 py-2 text-sm text-foreground hover:bg-muted"
                onClick={handleDetectSilence}
              >
                Test Silence Detect
              </button>
            </div>
          </div>
        )}
      </div>

      <div className="h-full w-96 flex-shrink-0 border-l border-border">
        <TranscriptPanel
          transcript={transcript}
          activeSegmentId={activeSegmentId}
          onSegmentClick={(id) => {
            const seg = transcript?.segments.find((s) => s.id === id);
            if (!seg) return;

            const seekResult = videoPreviewRef.current?.seek(seg.startMs);
            if (seekResult && seekResult.status !== "rejected" && seekResult.timeMs !== null) {
              setCurrentTime(seekResult.timeMs);
            } else if (!source || isSourceMissing) {
              setCurrentTime(seg.startMs);
            }
          }}
          onSegmentEdit={handleTranscriptEdit}
          onSegmentRevert={handleTranscriptRevert}
        />
      </div>
    </div>
  );
}
