import { useRef, useEffect, useMemo } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Transcript } from "@/types/transcript";
import { TranscriptSegment } from "./TranscriptSegment";

interface TranscriptPanelProps {
  transcript: Transcript | null;
  activeSegmentId?: string | null;
  selectedSegmentIds?: string[];
  cutSegmentIds?: string[];
  modifiedSegmentIds?: string[];
  onSegmentClick?: (id: string) => void;
  onSegmentEdit?: (id: string, newText: string) => void;
  autoScroll?: boolean;
}

export const TranscriptPanel = ({
  transcript,
  activeSegmentId,
  selectedSegmentIds = [],
  cutSegmentIds = [],
  modifiedSegmentIds = [],
  onSegmentClick,
  onSegmentEdit,
  autoScroll = true,
}: TranscriptPanelProps) => {
  const parentRef = useRef<HTMLDivElement>(null);

  const segments = useMemo(() => transcript?.segments || [], [transcript]);

  const rowVirtualizer = useVirtualizer({
    count: segments.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 64, // Approximate height of a segment
    overscan: 10,
  });

  // Auto-scroll to active segment
  useEffect(() => {
    if (autoScroll && activeSegmentId && transcript) {
      const index = segments.findIndex((s) => s.id === activeSegmentId);
      if (index !== -1) {
        rowVirtualizer.scrollToIndex(index, { align: "center", behavior: "smooth" });
      }
    }
  }, [activeSegmentId, autoScroll, rowVirtualizer, segments, transcript]);

  if (!transcript || segments.length === 0) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground p-8 text-center border-l bg-muted/20">
        <div className="flex flex-col items-center gap-2">
          <p className="text-sm">No transcript available.</p>
          <p className="text-xs">Run AI Transcription to generate segments.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full w-full max-w-sm border-l bg-background shadow-sm shrink-0">
      <div className="p-4 border-b shrink-0 flex items-center justify-between">
        <h3 className="font-semibold text-sm">Transcript</h3>
        <span className="text-xs text-muted-foreground">
          {segments.length} segments
        </span>
      </div>
      
      <div
        ref={parentRef}
        className="flex-1 overflow-y-auto overflow-x-hidden"
        style={{ scrollBehavior: "smooth" }}
      >
        <div
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
            width: "100%",
            position: "relative",
          }}
        >
          {rowVirtualizer.getVirtualItems().map((virtualItem) => {
            const segment = segments[virtualItem.index];
            return (
              <div
                key={segment.id}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  height: `${virtualItem.size}px`,
                  transform: `translateY(${virtualItem.start}px)`,
                }}
              >
                <TranscriptSegment
                  segment={segment}
                  isActive={segment.id === activeSegmentId}
                  isSelected={selectedSegmentIds.includes(segment.id)}
                  isCut={cutSegmentIds.includes(segment.id)}
                  isModified={modifiedSegmentIds.includes(segment.id)}
                  onClick={onSegmentClick}
                  onEdit={onSegmentEdit}
                />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
