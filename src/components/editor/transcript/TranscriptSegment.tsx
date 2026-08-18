import { memo, useState, useRef, useEffect, KeyboardEvent } from "react";
import { TranscriptSegment as ITranscriptSegment } from "@/types/transcript";
import { cn } from "@/lib/utils";

interface TranscriptSegmentProps {
  segment: ITranscriptSegment;
  isActive?: boolean;
  isSelected?: boolean;
  isCut?: boolean;
  isModified?: boolean;
  onClick?: (id: string) => void;
  onEdit?: (id: string, newText: string) => void;
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
    onEdit,
  }: TranscriptSegmentProps) => {
    const [isEditing, setIsEditing] = useState(false);
    const [editValue, setEditValue] = useState(segment.text);
    const inputRef = useRef<HTMLTextAreaElement>(null);

    useEffect(() => {
      if (isEditing && inputRef.current) {
        inputRef.current.focus();
        inputRef.current.setSelectionRange(
          inputRef.current.value.length,
          inputRef.current.value.length
        );
      }
    }, [isEditing]);

    useEffect(() => {
      if (!isEditing) {
        setEditValue(segment.text);
      }
    }, [segment.text, isEditing]);

    const handleSave = () => {
      if (editValue.trim() !== segment.text) {
        onEdit?.(segment.id, editValue.trim());
      }
      setIsEditing(false);
    };

    const handleCancel = () => {
      setEditValue(segment.text);
      setIsEditing(false);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSave();
      } else if (e.key === "Escape") {
        e.preventDefault();
        handleCancel();
      }
    };

    return (
      <div
        className={cn(
          "flex gap-4 p-2 mx-2 my-1 rounded-md transition-colors cursor-pointer group h-full",
          isActive ? "bg-primary/10" : "hover:bg-muted/50",
          isSelected && "ring-2 ring-primary ring-inset",
          isCut && "opacity-50 line-through"
        )}
        onClick={() => {
          if (!isEditing) onClick?.(segment.id);
        }}
        onDoubleClick={() => {
          if (!isCut) setIsEditing(true);
        }}
      >
        <div className="w-12 shrink-0 text-xs text-muted-foreground select-none font-mono mt-1 group-hover:text-foreground transition-colors">
          {formatTimestamp(segment.startMs)}
        </div>
        <div className="flex-1 text-sm md:text-base">
          {isEditing ? (
            <textarea
              ref={inputRef}
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              onBlur={handleSave}
              onKeyDown={handleKeyDown}
              className="w-full bg-background border border-input rounded-sm p-1 outline-none focus-visible:ring-1 focus-visible:ring-primary min-h-[40px] resize-none overflow-hidden"
              rows={Math.max(1, Math.ceil(editValue.length / 50))}
            />
          ) : (
            <span
              className={cn(
                "leading-relaxed block",
                (isModified || segment.isModified) && "text-amber-500 font-medium",
                !isCut && "group-hover:opacity-90"
              )}
              title="Double click to edit"
            >
              {segment.text}
            </span>
          )}
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
      prevProps.segment.text === nextProps.segment.text &&
      prevProps.segment.isModified === nextProps.segment.isModified
    );
  }
);
TranscriptSegment.displayName = "TranscriptSegment";
