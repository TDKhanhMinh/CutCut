import { useCallback, useEffect, useRef } from "react";
import { EditPlan } from "@/types/project";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/** A merged, sorted cut interval in source-time milliseconds. */
export interface CutInterval {
  startMs: number;
  endMs: number;
}

export interface CutSeekTarget {
  targetMs: number;
  pauseAtEnd: boolean;
}

/**
 * Tolerance (ms) used to detect re-entry into a cut region right after a seek.
 * Without this, the `timeupdate` event can fire before the browser commits the
 * seek, causing an infinite seek loop.
 */
const SEEK_TOLERANCE_MS = 150;

// ─────────────────────────────────────────────────────────────────────────────
// Cut-index builder
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Build a sorted, merged list of enabled cut intervals from a validated EditPlan.
 *
 * NOTE: The EditPlan arriving here has already been through `validate_and_normalize`,
 * so overlapping cuts are already merged on the Rust side. We still sort + merge
 * on the frontend for safety and to handle UI-only toggle state changes that
 * haven't been round-tripped through the validator yet.
 */
export function buildCutIndex(plan: EditPlan, sourceMediaId?: string): CutInterval[] {
  const candidateActions = plan.actions.filter(
    (action) => action.enabled && action.type === "cut" && action.sourceMediaId.trim().length > 0,
  );
  const sourceIds = new Set(candidateActions.map((action) => action.sourceMediaId));
  // Never let an unspecified preview source skip cuts from multiple media
  // sources. Single-source projects remain backwards compatible.
  const safeSourceId = sourceMediaId?.trim() || (sourceIds.size === 1 ? [...sourceIds][0] : null);
  const raw: CutInterval[] = plan.actions
    .filter(
      (action) =>
        action.enabled &&
        action.type === "cut" &&
        safeSourceId !== null &&
        action.sourceMediaId === safeSourceId &&
        Number.isFinite(action.startMs) &&
        Number.isFinite(action.endMs) &&
        action.startMs >= 0 &&
        action.endMs > action.startMs,
    )
    .map((action) => ({ startMs: action.startMs, endMs: action.endMs }))
    .sort((a, b) => a.startMs - b.startMs);

  // Merge overlapping/adjacent intervals (defensive — validator already did this)
  const merged: CutInterval[] = [];
  for (const interval of raw) {
    if (merged.length === 0) {
      merged.push({ ...interval });
      continue;
    }
    const last = merged[merged.length - 1];
    if (interval.startMs <= last.endMs) {
      last.endMs = Math.max(last.endMs, interval.endMs);
    } else {
      merged.push({ ...interval });
    }
  }
  return merged;
}

/**
 * Find the first cut interval that contains `timeMs`.
 * Returns `null` if no enabled cut covers the given time.
 */
export function findActiveCut(cutIndex: CutInterval[], timeMs: number): CutInterval | null {
  if (!Number.isFinite(timeMs)) return null;
  let low = 0;
  let high = cutIndex.length - 1;
  while (low <= high) {
    const middle = Math.floor((low + high) / 2);
    const cut = cutIndex[middle];
    if (timeMs < cut.startMs) {
      high = middle - 1;
    } else if (timeMs >= cut.endMs) {
      low = middle + 1;
    } else {
      return cut;
    }
  }
  return null;
}

/** Resolve a safe source-time seek for an active cut. */
export function resolveCutSeekTarget(
  cut: CutInterval,
  currentMs: number,
  durationMs: number | null,
): CutSeekTarget {
  const boundedEnd =
    durationMs !== null && Number.isFinite(durationMs)
      ? Math.min(Math.max(0, durationMs), cut.endMs)
      : Math.max(0, cut.endMs);
  const targetMs = Math.max(boundedEnd, Math.max(0, currentMs));
  return {
    targetMs,
    pauseAtEnd: durationMs !== null && Number.isFinite(durationMs) && targetMs >= durationMs,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Hook
// ─────────────────────────────────────────────────────────────────────────────

interface UseCutPreviewOptions {
  /** The <video> element to attach to. */
  videoRef: React.RefObject<HTMLVideoElement | null>;
  /** The current validated EditPlan (may be null when no project is open). */
  plan: EditPlan | null;
  /** Whether cut-preview mode is active. When false the hook is a no-op. */
  enabled: boolean;
  sourceMediaId?: string;
  /** Temporarily bypass cut skipping for suggestion context previews. */
  bypassRef?: React.MutableRefObject<boolean>;
}

/**
 * `useCutPreview` — non-destructive cut-preview controller.
 *
 * Attaches to a `<video>` element via `timeupdate` and skips over any
 * `enabled` cut from the EditPlan in real-time source-time playback.
 *
 * Design decisions:
 * - Cut index is rebuilt whenever `plan` changes (action toggle → instant update).
 * - Seek is guarded by `SEEK_TOLERANCE_MS` to break re-entry loops.
 * - The hook never writes to Project JSON or touches source media.
 * - Source time ≠ output/edited time; this is a preview approximation only.
 */
export function useCutPreview({
  videoRef,
  plan,
  enabled,
  sourceMediaId,
  bypassRef,
}: UseCutPreviewOptions) {
  // Mutable ref for cut index — avoids stale closures in timeupdate handler
  const cutIndexRef = useRef<CutInterval[]>([]);

  // Track when we last performed a skip seek (source-time ms) to break loops
  const lastSeekTargetRef = useRef<number | null>(null);

  // Rebuild cut index whenever the plan changes
  useEffect(() => {
    if (!plan || !enabled) {
      cutIndexRef.current = [];
      return;
    }
    cutIndexRef.current = buildCutIndex(plan, sourceMediaId);
  }, [plan, enabled, sourceMediaId]);

  const handleTimeUpdate = useCallback(() => {
    const video = videoRef.current;
    if (!video || !enabled || bypassRef?.current || cutIndexRef.current.length === 0) return;

    const currentMs = video.currentTime * 1000;

    // Loop protection: if we just seeked to `lastSeekTargetRef`, ignore events
    // until the playhead has moved at least SEEK_TOLERANCE_MS past the seek target.
    if (lastSeekTargetRef.current !== null) {
      const distFromLastSeek = Math.abs(currentMs - lastSeekTargetRef.current);
      if (distFromLastSeek < SEEK_TOLERANCE_MS) {
        return; // still in tolerance window — skip handling
      }
      // Tolerance window passed — clear the lock
      lastSeekTargetRef.current = null;
    }

    const activeCut = findActiveCut(cutIndexRef.current, currentMs);
    if (!activeCut) return;

    // Seek to the end of this cut
    const durationMs =
      Number.isFinite(video.duration) && video.duration >= 0 ? video.duration * 1000 : null;
    const resolved = resolveCutSeekTarget(activeCut, currentMs, durationMs);
    if (resolved.pauseAtEnd) {
      video.pause();
      if (durationMs !== null) video.currentTime = durationMs / 1000;
      lastSeekTargetRef.current = null;
      return;
    }

    // Perform the skip seek
    if (resolved.targetMs <= currentMs) return;
    lastSeekTargetRef.current = resolved.targetMs;
    video.currentTime = resolved.targetMs / 1000;
  }, [bypassRef, enabled, videoRef]);

  // Attach / detach the timeupdate listener
  useEffect(() => {
    const video = videoRef.current;
    if (!video || !enabled) return;

    video.addEventListener("timeupdate", handleTimeUpdate);
    return () => {
      video.removeEventListener("timeupdate", handleTimeUpdate);
    };
  }, [videoRef, handleTimeUpdate, enabled]);

  /**
   * Call this when the user manually seeks into a cut.
   * Per task spec: handle seek-into-cut with a stable policy.
   * We immediately skip to the cut end so the user lands past the cut.
   */
  const handleUserSeek = useCallback(() => {
    // After a manual seek, clear the loop-protection lock so timeupdate
    // can evaluate the new position freely. The handleTimeUpdate handler
    // will then detect and skip the cut if needed.
    lastSeekTargetRef.current = null;
    window.setTimeout(handleTimeUpdate, 0);
  }, [handleTimeUpdate]);

  return { handleUserSeek };
}
