import { convertFileSrc } from '@tauri-apps/api/core';
import { forwardRef, useImperativeHandle, useRef, useState } from 'react';
import { EditPlan, CaptionCue, CaptionStyle } from '@/types/project';
import { useCutPreview } from '@/hooks/useCutPreview';
import { Switch } from '@/components/ui/switch';
import { CaptionOverlay } from './CaptionOverlay';

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

export interface VideoPreviewRef {
    seekTo: (timeMs: number) => void;
    playRange: (startMs: number, endMs: number) => void;
}

interface VideoPreviewProps {
    path: string;
    editPlan?: EditPlan | null;
    captionCues?: CaptionCue[];
    captionStyle?: CaptionStyle | null;
}

// ─────────────────────────────────────────────────────────────────────────────
// Component
// ─────────────────────────────────────────────────────────────────────────────

export const VideoPreview = forwardRef<VideoPreviewRef, VideoPreviewProps>(
    ({ path, editPlan, captionCues, captionStyle }, ref) => {
        const videoRef = useRef<HTMLVideoElement>(null);
        const assetUrl = convertFileSrc(path);

        const rangeRef = useRef<{ startMs: number; endMs: number } | null>(null);

        const [cutPreviewActive, setCutPreviewActive] = useState(false);
        const [showCaptions, setShowCaptions] = useState(true);

        const { handleUserSeek } = useCutPreview({
            videoRef,
            plan: editPlan ?? null,
            enabled: cutPreviewActive,
        });

        useImperativeHandle(ref, () => ({
            seekTo: (timeMs: number) => {
                if (videoRef.current) {
                    rangeRef.current = null;
                    videoRef.current.currentTime = timeMs / 1000;
                }
            },
            playRange: (startMs: number, endMs: number) => {
                if (videoRef.current) {
                    const playStartMs = Math.max(0, startMs - 1000);
                    videoRef.current.currentTime = playStartMs / 1000;
                    rangeRef.current = { startMs, endMs };
                    videoRef.current.play().catch(console.error);
                }
            },
        }));

        const handleTimeUpdate = () => {
            if (!videoRef.current) return;

            if (rangeRef.current) {
                const currentMs = videoRef.current.currentTime * 1000;
                const { startMs, endMs } = rangeRef.current;

                if (currentMs >= startMs && currentMs < endMs) {
                    videoRef.current.currentTime = endMs / 1000;
                } else if (currentMs >= endMs + 1000) {
                    videoRef.current.pause();
                    rangeRef.current = null;
                }
            }
        };

        const hasCuts =
            editPlan !== null &&
            editPlan !== undefined &&
            editPlan.actions.some(a => a.enabled && a.payload.type === 'cut');
            
        const hasCaptions = captionCues && captionCues.length > 0;

        return (
            <div className="p-4 border rounded-lg bg-card text-card-foreground shadow-sm m-4 max-w-xl">
                <div className="flex items-center justify-between mb-2">
                    <h3 className="text-lg font-bold">Video Preview</h3>

                    <div className="flex items-center gap-4">
                        {hasCaptions && (
                            <div className="flex items-center gap-2">
                                <span className="text-sm text-muted-foreground">Captions</span>
                                <Switch
                                    id="captions-toggle"
                                    checked={showCaptions}
                                    onCheckedChange={setShowCaptions}
                                    aria-label="Toggle captions"
                                />
                            </div>
                        )}
                        {hasCuts && (
                            <div className="flex items-center gap-2">
                                <span className="text-sm text-muted-foreground">Preview Cuts</span>
                                <Switch
                                    id="cut-preview-toggle"
                                    checked={cutPreviewActive}
                                    onCheckedChange={setCutPreviewActive}
                                    aria-label="Toggle cut-preview mode"
                                />
                            </div>
                        )}
                    </div>
                </div>

                {cutPreviewActive && (
                    <p className="text-xs text-muted-foreground mb-2 italic">
                        Source-time preview — enabled cuts will be skipped.
                    </p>
                )}

                <div className="relative @container w-full bg-black rounded overflow-hidden" style={{ maxHeight: '400px' }}>
                    <video
                        ref={videoRef}
                        src={assetUrl}
                        controls
                        className="w-full h-full object-contain"
                        style={{ maxHeight: '400px' }}
                        onTimeUpdate={handleTimeUpdate}
                        onSeeked={handleUserSeek}
                    />
                    
                    {showCaptions && hasCaptions && (
                        <CaptionOverlay
                            videoRef={videoRef}
                            cues={captionCues}
                            styleModel={captionStyle ?? null}
                            editPlan={editPlan}
                        />
                    )}
                </div>
            </div>
        );
    }
);

VideoPreview.displayName = 'VideoPreview';
