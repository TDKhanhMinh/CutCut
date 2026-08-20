import { useMemo } from "react";
import { TranscriptSegment } from "@/types/transcript";

/**
 * Hook to find the active transcript segment based on current playback time.
 * Uses binary search for O(log N) performance.
 */
export const useTranscriptSync = (
  segments: TranscriptSegment[],
  currentTimeMs: number,
): string | null => {
  return useMemo(() => {
    if (!segments || segments.length === 0) return null;
    if (!Number.isFinite(currentTimeMs)) return null;

    // Find the last segment whose start is at or before playback time. This
    // makes an exact boundary belong to the newer segment and leaves gaps
    // inactive instead of incorrectly highlighting the previous segment.
    let low = 0;
    let high = segments.length;
    while (low < high) {
      const mid = Math.floor((low + high) / 2);
      if (segments[mid].startMs <= currentTimeMs) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }

    const candidate = segments[low - 1];
    if (candidate && currentTimeMs <= candidate.endMs) return candidate.id;

    // If time falls in a gap or outside the transcript, no segment is active.
    return null;
  }, [segments, currentTimeMs]);
};
