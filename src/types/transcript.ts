// Transcript is persisted as part of Project JSON. Re-export the canonical
// contract so editor-only consumers cannot drift on nullability or timestamp
// fields.
export type { Transcript, TranscriptSegment } from "./project";
