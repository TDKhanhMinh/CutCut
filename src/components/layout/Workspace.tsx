import { useState } from "react";
import { TranscriptPanel } from "../editor/transcript/TranscriptPanel";
import { Transcript } from "@/types/transcript";

const mockTranscript: Transcript = {
  id: "t1",
  sourceId: "s1",
  modelId: "m1",
  language: "vi",
  generatedAt: 0,
  segments: Array.from({ length: 1000 }).map((_, i) => ({
    id: `seg-${i}`,
    startMs: i * 2000,
    endMs: i * 2000 + 1500,
    text: `Đây là câu thoại tiếng Việt số ${i + 1} được ảo hoá mượt mà bởi Tanstack Virtual.`,
    isFiller: false,
  })),
};

export function Workspace() {
  const [activeSegmentId, setActiveSegmentId] = useState<string | null>("seg-5");

  return (
    <div className="flex h-full w-full overflow-hidden bg-background">
      <div className="flex-1 flex flex-col items-center justify-center bg-muted/10 p-4">
        <div className="max-w-md space-y-4 rounded-xl border border-dashed border-border bg-background p-8 text-center shadow-sm">
          <h3 className="text-lg font-medium">Video Preview Area</h3>
          <p className="text-sm text-muted-foreground">
            Clicking on transcript segments will trigger seeks here.
          </p>
          <div className="pt-4 flex items-center justify-center gap-2">
            <button
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md text-sm cursor-pointer hover:bg-primary/90"
              onClick={() => setActiveSegmentId(`seg-${Math.floor(Math.random() * 1000)}`)}
            >
              Random Active Segment
            </button>
          </div>
        </div>
      </div>

      <div className="w-96 flex-shrink-0 border-l border-border h-full">
        <TranscriptPanel
          transcript={mockTranscript}
          activeSegmentId={activeSegmentId}
          onSegmentClick={(id) => setActiveSegmentId(id)}
          onSegmentEdit={(id, newText) => console.log(`[Mock] Edited segment ${id}: ${newText}`)}
        />
      </div>
    </div>
  );
}
