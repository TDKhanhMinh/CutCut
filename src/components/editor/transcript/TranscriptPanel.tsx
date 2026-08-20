import { useRef, useEffect, useMemo, useState } from "react";
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
  onSegmentRevert?: (id: string) => void;
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
  onSegmentRevert,
  autoScroll = true,
}: TranscriptPanelProps) => {
  const parentRef = useRef<HTMLDivElement>(null);
  const [isAutoScrollEnabled, setIsAutoScrollEnabled] = useState(autoScroll);

  const segments = useMemo(() => transcript?.segments || [], [transcript]);

  // TanStack Virtual exposes imperative methods that React Compiler cannot memoize safely.
  // eslint-disable-next-line react-hooks/incompatible-library
  const rowVirtualizer = useVirtualizer({
    count: segments.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 64, // Approximate height of a segment
    overscan: 10,
  });

  // Auto-scroll to active segment
  useEffect(() => {
    if (isAutoScrollEnabled && activeSegmentId && transcript) {
      const index = segments.findIndex((s) => s.id === activeSegmentId);
      if (index !== -1) {
        rowVirtualizer.scrollToIndex(index, { align: "center", behavior: "smooth" });
      }
    }
  }, [activeSegmentId, isAutoScrollEnabled, rowVirtualizer, segments, transcript]);

  // Sync prop changes
  useEffect(() => {
    setIsAutoScrollEnabled(autoScroll);
  }, [autoScroll]);

  const handleSegmentClick = (id: string) => {
    // Re-enable auto-scroll when user explicitly clicks a segment
    setIsAutoScrollEnabled(true);
    onSegmentClick?.(id);
  };

  if (!transcript || segments.length === 0) {
    return (
      <div className="flex h-full items-center justify-center border-l bg-muted/20 p-8 text-center text-muted-foreground">
        <div className="flex flex-col items-center gap-2">
          <p className="text-sm">No transcript available.</p>
          <p className="text-xs">Run AI Transcription to generate segments.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="relative flex h-full w-full max-w-sm shrink-0 flex-col border-l bg-background shadow-sm">
      <div className="flex shrink-0 items-center justify-between border-b p-4">
        <h3 className="text-sm font-semibold">Transcript</h3>
        <span className="text-xs text-muted-foreground">{segments.length} segments</span>
      </div>

      {!isAutoScrollEnabled && activeSegmentId && (
        <div className="absolute right-4 top-16 z-10">
          <button
            onClick={() => setIsAutoScrollEnabled(true)}
            className="cursor-pointer rounded-full bg-primary/90 px-3 py-1.5 text-xs text-primary-foreground shadow-md transition-colors hover:bg-primary"
          >
            Resume Auto-scroll
          </button>
        </div>
      )}

      <div
        ref={parentRef}
        className="flex-1 overflow-y-auto overflow-x-hidden"
        style={{ scrollBehavior: "smooth" }}
        onWheel={() => setIsAutoScrollEnabled(false)}
        onTouchMove={() => setIsAutoScrollEnabled(false)}
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
                  onClick={handleSegmentClick}
                  onEdit={onSegmentEdit}
                  onRevert={onSegmentRevert}
                />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
