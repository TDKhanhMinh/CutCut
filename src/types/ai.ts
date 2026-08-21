export interface AIAnalysisRequest {
  requestId: string;
  sourceMediaId: string;
  segments: Array<{ id: string; startMs: number; endMs: number; text: string }>;
  config: {
    language: string;
    strictMode: boolean;
    maxTokens?: number;
    temperature?: number;
  };
  instructions?: string;
}

export interface AIAnalysisResponse {
  actions: Array<{
    id: string;
    sourceMediaId: string;
    startMs: number;
    endMs: number;
    action: "CUT" | "KEEP" | "HIGHLIGHT";
    reason: string;
    confidence: number;
    taxonomy: string;
    source: "ai";
    segmentIds: string[];
  }>;
  summary?: string | null;
  usageTokens?: number | null;
  provider?: string | null;
  model?: string | null;
  promptVersion?: string | null;
}
