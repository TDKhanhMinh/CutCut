import type { ArtifactRecord } from "./artifact";
import type { MediaSourceMetadata } from "@/types/media";
import type { SilenceConfig } from "./silence";

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
  captions: CaptionStyle | null;
  /** Optional because older Rust project files do not contain caption artifacts. */
  captionCues?: CaptionCue[];
  /** Optional UI preference; detector settings are never required to load a project. */
  silenceSettings?: SilenceConfig;
  settings: OutputSettings;
  artifacts: ArtifactRecord[];
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
  originalText?: string;
  startMs: number;
  endMs: number;
  speaker: string | null;
  isFiller: boolean;
  isModified?: boolean;
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

export interface CaptionStyle {
  presetId: string;
  fontFamily: string;
  fontWeight: number;
  fontStyle: string;
  fontSizeVh: number;
  positionXVw: number;
  positionYVh: number;
  alignment: string;
  primaryColor: string;
  outlineColor: string | null;
  outlineWidthVh: number | null;
  backgroundColor: string | null;
  backgroundOpacity: number | null;
}

export interface CaptionCue {
  id: string;
  sourceSegmentIds: string[];
  startMs: number;
  endMs: number;
  text: string;
  isManualModified: boolean;
}

/** Legacy/simple caption shape kept for callers that only have preset values. */
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
