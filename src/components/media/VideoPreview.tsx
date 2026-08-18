import { convertFileSrc } from '@tauri-apps/api/core';
import { forwardRef, useImperativeHandle, useRef } from 'react';

export interface VideoPreviewRef {
    seekTo: (timeMs: number) => void;
    playRange: (startMs: number, endMs: number) => void;
}

interface VideoPreviewProps {
    path: string;
}

export const VideoPreview = forwardRef<VideoPreviewRef, VideoPreviewProps>(({ path }, ref) => {
    const videoRef = useRef<HTMLVideoElement>(null);
    const assetUrl = convertFileSrc(path);
    
    // Store active range playback info
    const rangeRef = useRef<{ startMs: number, endMs: number } | null>(null);

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
                // Start playback 1 second before the cut
                const playStartMs = Math.max(0, startMs - 1000);
                videoRef.current.currentTime = playStartMs / 1000;
                rangeRef.current = { startMs, endMs };
                videoRef.current.play().catch(console.error);
            }
        }
    }));

    const handleTimeUpdate = () => {
        if (!videoRef.current || !rangeRef.current) return;
        
        const currentMs = videoRef.current.currentTime * 1000;
        const { startMs, endMs } = rangeRef.current;
        
        // If we reach the cut start, jump over the cut to the end
        if (currentMs >= startMs && currentMs < endMs) {
            videoRef.current.currentTime = endMs / 1000;
        } 
        // Stop playback 1 second after the cut ends
        else if (currentMs >= endMs + 1000) {
            videoRef.current.pause();
            rangeRef.current = null;
        }
    };

    return (
        <div className="p-4 border rounded-lg bg-card text-card-foreground shadow-sm m-4 max-w-xl">
            <h3 className="text-lg font-bold mb-2">Video Preview</h3>
            <video 
                ref={videoRef}
                src={assetUrl} 
                controls 
                className="w-full bg-black rounded"
                style={{ maxHeight: '400px' }}
                onTimeUpdate={handleTimeUpdate}
            />
        </div>
    );
});
