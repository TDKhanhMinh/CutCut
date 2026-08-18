export interface TranscriptSegment {
  id: string;
  text: string;
  startMs: number;
  endMs: number;
  speaker?: string;
  isFiller: boolean;
}

export interface Transcript {
  id: string;
  sourceId: string;
  modelId: string;
  language: string;
  generatedAt: number;
  segments: TranscriptSegment[];
}
