import type { ArtifactRecord } from "@/types/artifact";
import type { TranscriptSegment } from "@/types/project";

/**
 * Transcript text is plain text today. Keep Unicode (including Vietnamese
 * combining marks) intact while removing layout-only whitespace that caption
 * consumers cannot represent consistently.
 */
export function normalizeTranscriptText(value: string): string {
  return value.replace(/\s+/gu, " ").trim();
}

/**
 * User edits change the transcript meaning, so generated transcript and
 * transcript-dependent semantic/rendering artifacts must be recomputed. We
 * only mark metadata stale; no caption cue/timestamp is rewritten here. A
 * detached caption therefore remains untouched in content and is surfaced as
 * stale for an explicit re-generation decision.
 */
export function markTranscriptDependentArtifactsStale(
  artifacts: ArtifactRecord[],
): ArtifactRecord[] {
  const staleTypes = new Set(["transcript", "localAnalysis", "aiAnalysis", "caption", "preview"]);

  return artifacts.map((artifact) =>
    staleTypes.has(artifact.artifactType)
      ? {
          ...artifact,
          status: artifact.status === "missing" ? artifact.status : "stale",
          diagnosticReason:
            artifact.status === "missing" ? artifact.diagnosticReason : "dependencyChanged",
        }
      : artifact,
  );
}

export interface TranscriptTextEditResult {
  segment: TranscriptSegment;
  changed: boolean;
}

/** Apply only a text edit; timing and identity are copied, never recalculated. */
export function applyTranscriptTextEdit(
  segment: TranscriptSegment,
  value: string,
): TranscriptTextEditResult {
  const nextText = normalizeTranscriptText(value);
  if (!nextText || nextText === segment.text) {
    return { segment, changed: false };
  }

  return {
    changed: true,
    segment: {
      ...segment,
      text: nextText,
      originalText: segment.originalText ?? segment.text,
      isModified: true,
    },
  };
}

export function revertTranscriptTextEdit(segment: TranscriptSegment): TranscriptSegment {
  if (segment.originalText === undefined) return segment;
  return {
    ...segment,
    text: segment.originalText,
    isModified: false,
  };
}
