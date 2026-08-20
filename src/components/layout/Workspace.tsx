import { useState, useEffect } from "react";
import { TranscriptPanel } from "../editor/transcript/TranscriptPanel";
import { Transcript } from "@/types/transcript";
import { useTranscriptSync } from "@/hooks/useTranscriptSync";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { SilenceInterval } from "@/types/silence";

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
    isFiller: false,
  })),
};

export function Workspace() {
  const [currentTime, setCurrentTime] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);

  const activeSegmentId = useTranscriptSync(mockTranscript.segments, currentTime);

  useEffect(() => {
    if (!isPlaying) return;
    const interval = setInterval(() => {
      setCurrentTime((prev) => prev + 100);
    }, 100);
    return () => clearInterval(interval);
  }, [isPlaying]);

  const handleDetectSilence = async () => {
    try {
      const jobId = `silence-${Date.now()}`;

      const unlisten = await listen(
        "media-job",
        (event: {
          payload: { jobId: string; state: string; message?: string; error?: string };
        }) => {
          const payload = event.payload;
          if (payload.jobId !== jobId) return;

          if (payload.state === "Completed") {
            const intervals: SilenceInterval[] = JSON.parse(payload.message || "[]");
            console.log("Detected silence intervals:", intervals);
            alert(`Found ${intervals.length} silence intervals! Check console.`);
            unlisten();
          } else if (payload.state === "Failed" || payload.state === "Cancelled") {
            console.error("Silence detection failed/cancelled:", payload.error || payload.message);
            alert("Silence detection failed: " + (payload.error || payload.message));
            unlisten();
          }
        },
      );

      // Pass a dummy path; it will fail, but proves the IPC works.
      await invoke("start_silence_detection", {
        jobId,
        path: "C:\\non_existent.mp4",
        settings: {
          thresholdDb: -35,
          minDurationMs: 500,
        },
      });
    } catch (e) {
      console.error("IPC Error:", e);
    }
  };

  return (
    <div className="flex h-full w-full overflow-hidden bg-background">
      <div className="flex flex-1 flex-col items-center justify-center bg-muted/10 p-4">
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
      </div>

      <div className="h-full w-96 flex-shrink-0 border-l border-border">
        <TranscriptPanel
          transcript={mockTranscript}
          activeSegmentId={activeSegmentId}
          onSegmentClick={(id) => {
            const seg = mockTranscript.segments.find((s) => s.id === id);
            if (seg) setCurrentTime(seg.startMs);
          }}
          onSegmentEdit={(id, newText) => console.log(`[Mock] Edited segment ${id}: ${newText}`)}
        />
      </div>
    </div>
  );
}
