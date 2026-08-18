import { MediaSourceMetadata } from "../components/media/MediaImporter";
import { SilenceConfig } from "./silence";

export const CURRENT_SCHEMA_VERSION = 1;

export interface Project {
    id: string;
    schemaVersion: number;
    createdAt: number;
    updatedAt: number;
    media: MediaSource[];
    transcript: Transcript | null;
    edits: EditTimeline;
    captions: CaptionSettings | null;
    settings: OutputSettings;
    silenceSettings: SilenceConfig;
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

export interface EditTimeline {
    actions: EditAction[];
}

export type EditAction = 
    | CutAction
    | KeepAction
    | MuteAction;

export interface CutAction {
    type: 'Cut';
    id: string;
    sourceMediaId: string;
    startMs: number;
    endMs: number;
}

export interface KeepAction {
    type: 'Keep';
    id: string;
    sourceMediaId: string;
    startMs: number;
    endMs: number;
}

export interface MuteAction {
    type: 'Mute';
    id: string;
    sourceMediaId: string;
    startMs: number;
    endMs: number;
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
