import { useState, useEffect } from "react";
import { TranscriptPanel } from "../editor/transcript/TranscriptPanel";
import { Transcript } from "@/types/transcript";
import { useTranscriptSync } from "@/hooks/useTranscriptSync";

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

  return (
    <div className="flex h-full w-full overflow-hidden bg-background">
      <div className="flex-1 flex flex-col items-center justify-center bg-muted/10 p-4">
        <div className="max-w-md space-y-4 rounded-xl border border-dashed border-border bg-background p-8 text-center shadow-sm">
          <h3 className="text-lg font-medium">Video Preview Area</h3>
          <p className="text-sm text-muted-foreground">
            Current simulated time: {(currentTime / 1000).toFixed(1)}s
          </p>
          <div className="pt-4 flex items-center justify-center gap-2">
            <button
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md text-sm cursor-pointer hover:bg-primary/90"
              onClick={() => setIsPlaying(!isPlaying)}
            >
              {isPlaying ? "Pause Playback" : "Start Playback"}
            </button>
            <button
              className="px-4 py-2 bg-secondary text-secondary-foreground rounded-md text-sm cursor-pointer hover:bg-secondary/90"
              onClick={() => setCurrentTime((prev) => prev + 10000)}
            >
              +10s Seek
            </button>
          </div>
        </div>
      </div>

      <div className="w-96 flex-shrink-0 border-l border-border h-full">
        <TranscriptPanel
          transcript={mockTranscript}
          activeSegmentId={activeSegmentId}
          onSegmentClick={(id) => {
            const seg = mockTranscript.segments.find(s => s.id === id);
            if (seg) setCurrentTime(seg.startMs);
          }}
          onSegmentEdit={(id, newText) => console.log(`[Mock] Edited segment ${id}: ${newText}`)}
        />
      </div>
    </div>
  );
}
