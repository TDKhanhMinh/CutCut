export interface ProjectSettings {
  resolution: [number, number];
  fps: number;
}

export interface Project {
  id: string;
  name: string;
  version: string;
  source_media_path: string | null;
  settings: ProjectSettings;
}

export interface TranscriptSegment {
  id: string;
  start_ms: number;
  end_ms: number;
  text: string;
}

export interface Transcript {
  id: string;
  segments: TranscriptSegment[];
}

export type ActionSource = "Local" | "AI" | "User";

export type ActionType = 
  | { Cut: null } 
  | { Mute: null } 
  | { Zoom: number };

export interface EditAction {
  id: string;
  action_type: ActionType;
  start_ms: number;
  end_ms: number;
  source: ActionSource;
  enabled: boolean;
}

export interface EditPlan {
  actions: EditAction[];
}
