import { memo } from "react";
import { TranscriptSegment as ITranscriptSegment } from "@/types/transcript";
import { cn } from "@/lib/utils";

interface TranscriptSegmentProps {
  segment: ITranscriptSegment;
  isActive?: boolean;
  isSelected?: boolean;
  isCut?: boolean;
  isModified?: boolean;
  onClick?: (id: string) => void;
}

const formatTimestamp = (ms: number) => {
  const totalSeconds = Math.floor(ms / 1000);
  const m = Math.floor(totalSeconds / 60);
  const s = totalSeconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
};

export const TranscriptSegment = memo(
  ({
    segment,
    isActive,
    isSelected,
    isCut,
    isModified,
    onClick,
  }: TranscriptSegmentProps) => {
    return (
      <div
        className={cn(
          "flex gap-4 p-2 mx-2 my-1 rounded-md transition-colors cursor-pointer group h-full",
          isActive ? "bg-primary/10" : "hover:bg-muted/50",
          isSelected && "ring-2 ring-primary ring-inset",
          isCut && "opacity-50 line-through"
        )}
        onClick={() => onClick?.(segment.id)}
      >
        <div className="w-12 shrink-0 text-xs text-muted-foreground select-none font-mono mt-1 group-hover:text-foreground transition-colors">
          {formatTimestamp(segment.startMs)}
        </div>
        <div className="flex-1 text-sm md:text-base">
          <span
            className={cn(
              "leading-relaxed block",
              isModified && "text-amber-500 font-medium"
            )}
          >
            {segment.text}
          </span>
        </div>
      </div>
    );
  },
  (prevProps, nextProps) => {
    return (
      prevProps.isActive === nextProps.isActive &&
      prevProps.isSelected === nextProps.isSelected &&
      prevProps.isCut === nextProps.isCut &&
      prevProps.isModified === nextProps.isModified &&
      prevProps.segment.id === nextProps.segment.id &&
      prevProps.segment.text === nextProps.segment.text
    );
  }
);
TranscriptSegment.displayName = "TranscriptSegment";
