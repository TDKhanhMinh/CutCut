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

    let left = 0;
    let right = segments.length - 1;

    // Fast bounds checking
    if (currentTimeMs < segments[0].startMs) return null;
    if (currentTimeMs > segments[segments.length - 1].endMs) return null;

    while (left <= right) {
      const mid = Math.floor((left + right) / 2);
      const segment = segments[mid];

      if (currentTimeMs >= segment.startMs && currentTimeMs <= segment.endMs) {
        return segment.id;
      }

      if (currentTimeMs < segment.startMs) {
        right = mid - 1;
      } else {
        left = mid + 1;
      }
    }

    // If time falls in a gap between segments, we return null so no segment is active.
    return null;
  }, [segments, currentTimeMs]);
};
