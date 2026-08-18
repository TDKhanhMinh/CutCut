export interface TranscriptSegment {
  id: string;
  text: string;
  originalText?: string;
  startMs: number;
  endMs: number;
  speaker?: string;
  isFiller: boolean;
  isModified?: boolean;
}

export interface Transcript {
  id: string;
  sourceId: string;
  modelId: string;
  language: string;
  generatedAt: number;
  segments: TranscriptSegment[];
}
