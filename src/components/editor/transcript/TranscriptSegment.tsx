import { memo, useState, useRef, useEffect, KeyboardEvent } from "react";
import { TranscriptSegment as ITranscriptSegment } from "@/types/transcript";
import { cn } from "@/lib/utils";
import { normalizeTranscriptText } from "@/lib/transcript-edit";

interface TranscriptSegmentProps {
  segment: ITranscriptSegment;
  isActive?: boolean;
  isSelected?: boolean;
  isCut?: boolean;
  isModified?: boolean;
  onClick?: (id: string) => void;
  onEdit?: (id: string, newText: string) => void;
  onRevert?: (id: string) => void;
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
    onRevert,
  }: TranscriptSegmentProps) => {
    const [isEditing, setIsEditing] = useState(false);
    const [editValue, setEditValue] = useState(segment.text);
    const inputRef = useRef<HTMLTextAreaElement>(null);
    const clickTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const editingRef = useRef(false);

    useEffect(() => {
      if (isEditing && inputRef.current) {
        inputRef.current.focus();
        inputRef.current.setSelectionRange(
          inputRef.current.value.length,
          inputRef.current.value.length,
        );
      }
    }, [isEditing]);

    useEffect(() => {
      if (!isEditing) {
        setEditValue(segment.text);
      }
    }, [segment.text, isEditing]);

    useEffect(() => {
      return () => {
        if (clickTimerRef.current) clearTimeout(clickTimerRef.current);
      };
    }, []);

    const startEditing = () => {
      if (isCut || editingRef.current) return;
      editingRef.current = true;
      setEditValue(segment.text);
      setIsEditing(true);
    };

    const handleSave = () => {
      if (!editingRef.current) return;
      editingRef.current = false;
      const normalizedText = normalizeTranscriptText(editValue);
      if (normalizedText && normalizedText !== segment.text) {
        onEdit?.(segment.id, normalizedText);
      }
      setIsEditing(false);
    };

    const handleCancel = () => {
      if (!editingRef.current) return;
      editingRef.current = false;
      setEditValue(segment.text);
      setIsEditing(false);
    };

    const handleClick = () => {
      if (isEditing) return;
      if (clickTimerRef.current) clearTimeout(clickTimerRef.current);
      clickTimerRef.current = setTimeout(() => {
        clickTimerRef.current = null;
        onClick?.(segment.id);
      }, 220);
    };

    const handleDoubleClick = () => {
      if (clickTimerRef.current) {
        clearTimeout(clickTimerRef.current);
        clickTimerRef.current = null;
      }
      startEditing();
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
          "group mx-2 my-1 flex h-full cursor-pointer gap-4 rounded-md p-2 outline-none transition-colors focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary",
          isActive ? "bg-primary/10" : "hover:bg-muted/50 focus-visible:bg-muted/50",
          isSelected && "ring-2 ring-inset ring-primary",
          isCut && "line-through opacity-50",
        )}
        tabIndex={0}
        onKeyDown={(e) => {
          if (
            e.target instanceof HTMLElement &&
            ["BUTTON", "INPUT", "TEXTAREA"].includes(e.target.tagName)
          ) {
            return;
          }
          if (!isEditing && (e.key === "Enter" || e.key === " ")) {
            e.preventDefault();
            onClick?.(segment.id);
          }
        }}
        onClick={handleClick}
        onDoubleClick={handleDoubleClick}
      >
        <div className="mt-1 w-12 shrink-0 select-none font-mono text-xs text-muted-foreground transition-colors group-hover:text-foreground">
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
              className="min-h-[40px] w-full resize-none overflow-hidden rounded-sm border border-input bg-background p-1 outline-none focus-visible:ring-1 focus-visible:ring-primary"
              rows={Math.max(1, Math.ceil(editValue.length / 50))}
            />
          ) : (
            <div className="flex items-start gap-2">
              <span
                className={cn(
                  "block flex-1 leading-relaxed",
                  (isModified || segment.isModified) && "font-medium text-amber-500",
                  !isCut && "group-hover:opacity-90",
                )}
                title="Double click to edit"
              >
                {segment.text}
              </span>
              {segment.isModified && onRevert && (
                <button
                  type="button"
                  className="shrink-0 text-xs text-muted-foreground underline-offset-2 hover:text-foreground hover:underline"
                  aria-label={`Revert transcript segment ${segment.id}`}
                  onClick={(event) => {
                    event.stopPropagation();
                    onRevert(segment.id);
                  }}
                  onDoubleClick={(event) => event.stopPropagation()}
                >
                  Revert
                </button>
              )}
            </div>
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
      prevProps.onEdit === nextProps.onEdit &&
      prevProps.onRevert === nextProps.onRevert &&
      prevProps.segment.id === nextProps.segment.id &&
      prevProps.segment.text === nextProps.segment.text &&
      prevProps.segment.isModified === nextProps.segment.isModified &&
      prevProps.segment.originalText === nextProps.segment.originalText
    );
  },
);
TranscriptSegment.displayName = "TranscriptSegment";
