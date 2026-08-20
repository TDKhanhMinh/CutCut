import type { MediaSourceMetadata } from "@/types/media";

export const CURRENT_SCHEMA_VERSION = 1;

/** Canonical Project JSON contract. All timestamps use milliseconds. */
export interface Project {
  id: string;
  schemaVersion: number;
  createdAt: number;
  updatedAt: number;
  media: MediaSource[];
  transcript: Transcript | null;
  editPlan: EditPlan;
  captions: CaptionSettings | null;
  settings: OutputSettings;
}

export interface MediaSource {
  id: string;
  path: string;
  metadata: MediaSourceMetadata;
}

export interface Transcript {
  id: string;
  sourceId: string;
  modelId: string;
  language: string;
  generatedAt: number;
  segments: TranscriptSegment[];
}

export interface TranscriptSegment {
  id: string;
  text: string;
  startMs: number;
  endMs: number;
  speaker: string | null;
  isFiller: boolean;
}

export interface EditPlan {
  actions: EditAction[];
}

export type EditActionType = "cut" | "keep" | "mute";
export type EditActionSource = "local" | "ai" | "user";

export interface EditAction {
  id: string;
  type: EditActionType;
  sourceMediaId: string;
  startMs: number;
  endMs: number;
  source: EditActionSource;
  reason: string;
  confidence: number | null;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface CaptionSettings {
  style: string;
  fontSize: number;
  primaryColor: string;
  strokeColor: string;
}

export interface OutputSettings {
  aspectRatio: string;
  targetResolution: number;
  fps: number;
}
