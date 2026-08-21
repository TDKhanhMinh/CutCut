import { invoke } from "@tauri-apps/api/core";
import { authService } from "@/services/auth";
import { useAIConfigStore } from "@/stores/useAIConfigStore";
import type { AIAnalysisRequest, AIAnalysisResponse } from "@/types/ai";
import type { EditAction, EditPlan, MediaSource, Transcript } from "@/types/project";
import { validateEditPlan } from "@/services/project";

function requestId(): string {
  return (
    globalThis.crypto?.randomUUID?.() ?? `ai-${Date.now()}-${Math.random().toString(36).slice(2)}`
  );
}

function toRequest(mediaId: string, transcript: Transcript): AIAnalysisRequest {
  return {
    requestId: requestId(),
    sourceMediaId: mediaId,
    segments: transcript.segments.map((segment) => ({
      id: segment.id,
      startMs: segment.startMs,
      endMs: segment.endMs,
      text: segment.text,
    })),
    config: {
      language: transcript.language,
      strictMode: true,
      maxTokens: 4096,
      temperature: 0.1,
    },
  };
}

export async function analyzeTranscript(
  mediaId: string,
  transcript: Transcript,
): Promise<AIAnalysisResponse> {
  const request = toRequest(mediaId, transcript);
  if (useAIConfigStore.getState().mode === "byok") {
    return invoke<AIAnalysisResponse>("call_gemini_direct", { request });
  }
  const { data, error } = await authService.invokeFunction<AIAnalysisResponse>(
    "ai-analyze",
    request as unknown as Record<string, unknown>,
  );
  if (error || !data) throw new Error(error?.message ?? "provider_unavailable");
  return data;
}

export function mergeAIResponse(editPlan: EditPlan, response: AIAnalysisResponse): EditPlan {
  const now = Date.now();
  const actions: EditAction[] = response.actions.map((action) => ({
    id: action.id,
    type: action.action.toLowerCase() as EditAction["type"],
    sourceMediaId: action.sourceMediaId,
    startMs: action.startMs,
    endMs: action.endMs,
    source: "ai",
    reason: action.reason,
    confidence: action.confidence,
    enabled: action.action !== "CUT",
    createdAt: now,
    updatedAt: now,
  }));
  const generatedIds = new Set(actions.map((action) => action.id));
  return {
    ...editPlan,
    actions: [...editPlan.actions.filter((action) => !generatedIds.has(action.id)), ...actions],
  };
}

export async function analyzeAndMerge(
  mediaId: string,
  transcript: Transcript,
  editPlan: EditPlan,
  media: MediaSource[],
): Promise<EditPlan> {
  const nextPlan = mergeAIResponse(editPlan, await analyzeTranscript(mediaId, transcript));
  const validation = await validateEditPlan(nextPlan, media);
  const errors = validation.issues.filter((issue) => issue.level === "error");
  if (errors.length > 0) throw new Error(errors.map((issue) => issue.message).join("; "));
  return nextPlan;
}
