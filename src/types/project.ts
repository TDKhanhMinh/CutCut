import { MediaSourceMetadata } from "../components/media/MediaImporter";
import { SilenceConfig } from "./silence";
import { ArtifactRecord } from './artifact';

export const CURRENT_SCHEMA_VERSION = 1;

export interface Project {
    id: string;
    schemaVersion: number;
    createdAt: number;
    updatedAt: number;
    media: MediaSource[];
    transcript: Transcript | null;
    editPlan: EditPlan;
    captions: CaptionStyle | null;
    captionCues: CaptionCue[];
    settings: OutputSettings;
    silenceSettings: SilenceConfig;
    artifacts: ArtifactRecord[];
}

export interface CaptionStyle {
    presetId: string;
    
    // Typography
    fontFamily: string;
    fontWeight: number;
    fontStyle: string;
    
    // Geometry
    fontSizeVh: number;
    positionXVw: number;
    positionYVh: number;
    alignment: string;
    
    // Colors & FX
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


export interface MediaSource {
    id: string;
    path: string;
    metadata: MediaSourceMetadata;
}

export interface Transcript {
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
    version: number;
    actions: EditAction[];
    generationMetadata?: GenerationMetadata | null;
}

export interface GenerationMetadata {
    analyzerVersion?: string | null;
    modelId?: string | null;
    runId?: string | null;
}

export type ActionSource = 'localDetector' | 'aiAgent' | 'userManual';

export type ActionPayload = 
    | CutPayload
    | ZoomPayload
    | HighlightPayload
    | CaptionPayload;

export interface CutPayload {
    type: 'cut';
    startMs: number;
    endMs: number;
}

export interface HighlightPayload {
    type: 'highlight';
    startMs: number;
    endMs: number;
}

export interface ZoomPayload {
    type: 'zoom';
    startMs: number;
    endMs: number;
    scale: number;
    anchorX: number;
    anchorY: number;
    easing: string;
}

export interface CaptionPayload {
    type: 'caption';
    startMs: number;
    endMs: number;
    text: string;
    styleReference?: string | null;
}

export interface EditAction {
    id: string;
    sourceMediaId: string;
    payload: ActionPayload;
    source: ActionSource;
    reason: string;
    confidence?: string | null;
    enabled: boolean;
    isManualModified?: boolean;
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
