import type { CaptionCue } from '@/types/project';

export function sortCaptionCues(cues: CaptionCue[]): CaptionCue[] {
  return [...cues].filter((cue) => cue.endMs > cue.startMs).sort((a, b) => a.startMs - b.startMs);
}

export function findActiveCaptionCue(
  sortedCues: CaptionCue[],
  currentTimeMs: number,
): CaptionCue | null {
  if (!Number.isFinite(currentTimeMs)) return null;
  let low = 0;
  let high = sortedCues.length - 1;
  while (low <= high) {
    const middle = Math.floor((low + high) / 2);
    const cue = sortedCues[middle];
    if (currentTimeMs < cue.startMs) high = middle - 1;
    else if (currentTimeMs >= cue.endMs) low = middle + 1;
    else return cue;
  }
  return null;
}
