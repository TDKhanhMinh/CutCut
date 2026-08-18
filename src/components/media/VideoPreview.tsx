import { convertFileSrc } from '@tauri-apps/api/core';
import { forwardRef, useImperativeHandle, useRef, useState } from 'react';
import { EditPlan } from '@/types/project';
import { useCutPreview } from '@/hooks/useCutPreview';
import { Switch } from '@/components/ui/switch';

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

export interface VideoPreviewRef {
    seekTo: (timeMs: number) => void;
    playRange: (startMs: number, endMs: number) => void;
}

interface VideoPreviewProps {
    path: string;
    /**
     * The validated EditPlan to use for cut-preview mode.
     * When provided, the player will automatically skip `enabled` cuts during playback.
     */
    editPlan?: EditPlan | null;
}

// ─────────────────────────────────────────────────────────────────────────────
// Component
// ─────────────────────────────────────────────────────────────────────────────

export const VideoPreview = forwardRef<VideoPreviewRef, VideoPreviewProps>(
    ({ path, editPlan }, ref) => {
        const videoRef = useRef<HTMLVideoElement>(null);
        const assetUrl = convertFileSrc(path);

        // Store active range playback info (for suggestion preview)
        const rangeRef = useRef<{ startMs: number; endMs: number } | null>(null);

        // User-controllable toggle for cut-preview mode
        const [cutPreviewActive, setCutPreviewActive] = useState(false);

        // Attach non-destructive cut-preview controller
        const { handleUserSeek } = useCutPreview({
            videoRef,
            plan: editPlan ?? null,
            enabled: cutPreviewActive,
        });

        useImperativeHandle(ref, () => ({
            seekTo: (timeMs: number) => {
                if (videoRef.current) {
                    // Cancel any active range playback
                    rangeRef.current = null;
                    videoRef.current.currentTime = timeMs / 1000;
                }
            },
            playRange: (startMs: number, endMs: number) => {
                if (videoRef.current) {
                    // Start playback 1 second before the cut for context
                    const playStartMs = Math.max(0, startMs - 1000);
                    videoRef.current.currentTime = playStartMs / 1000;
                    rangeRef.current = { startMs, endMs };
                    videoRef.current.play().catch(console.error);
                }
            },
        }));

        const handleTimeUpdate = () => {
            if (!videoRef.current) return;

            // --- Suggestion range preview (bypass cut-preview mode) ---
            if (rangeRef.current) {
                const currentMs = videoRef.current.currentTime * 1000;
                const { startMs, endMs } = rangeRef.current;

                // Skip over the cut to show "before → after" context
                if (currentMs >= startMs && currentMs < endMs) {
                    videoRef.current.currentTime = endMs / 1000;
                } else if (currentMs >= endMs + 1000) {
                    // Stop 1s after cut ends
                    videoRef.current.pause();
                    rangeRef.current = null;
                }
            }
            // Note: useCutPreview attaches its own timeupdate listener separately.
            // Both can coexist because rangeRef takes priority (checked first).
        };

        const hasCuts =
            editPlan !== null &&
            editPlan !== undefined &&
            editPlan.actions.some(a => a.enabled && a.payload.type === 'cut');

        return (
            <div className="p-4 border rounded-lg bg-card text-card-foreground shadow-sm m-4 max-w-xl">
                <div className="flex items-center justify-between mb-2">
                    <h3 className="text-lg font-bold">Video Preview</h3>

                    {/* Cut-preview toggle — only shown when plan has enabled cuts */}
                    {hasCuts && (
                        <div className="flex items-center gap-2">
                            <span className="text-sm text-muted-foreground">
                                Preview Cuts
                            </span>
                            <Switch
                                id="cut-preview-toggle"
                                checked={cutPreviewActive}
                                onCheckedChange={setCutPreviewActive}
                                aria-label="Toggle cut-preview mode"
                            />
                        </div>
                    )}
                </div>

                {cutPreviewActive && (
                    <p className="text-xs text-muted-foreground mb-2 italic">
                        Source-time preview — enabled cuts will be skipped during playback.
                        This is an approximation; source file is not modified.
                    </p>
                )}

                <video
                    ref={videoRef}
                    src={assetUrl}
                    controls
                    className="w-full bg-black rounded"
                    style={{ maxHeight: '400px' }}
                    onTimeUpdate={handleTimeUpdate}
                    onSeeked={handleUserSeek}
                />
            </div>
        );
    }
);

VideoPreview.displayName = 'VideoPreview';
