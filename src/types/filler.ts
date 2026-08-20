import type { EditAction } from "@/types/project";

export type TimestampPrecision = "wordLevel" | "segmentLevel";

export interface FillerCandidate {
  id: string;
  sourceMediaId: string;
  matchedToken: string;
  segmentText: string;
  startMs: number;
  endMs: number;
  precision: TimestampPrecision;
  reviewRequired: boolean;
}

export interface FillerAnalysisResult {
  dictionaryVersion: string;
  candidates: FillerCandidate[];
  actions: EditAction[];
}
