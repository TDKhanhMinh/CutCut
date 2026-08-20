import { useState } from "react";
import { CutSuggestion, SuggestionCard } from "./SuggestionCard";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import type { EditPlan } from "@/types/project";
import type { NonSpeechCandidate } from "@/types/fusion";
import { DEFAULT_SILENCE_CONFIG, type SilenceConfig } from "@/types/silence";
import { analyzeNonSpeech } from "@/services/nonSpeechAnalysis";

interface ReviewControlsProps {
  mediaId: string;
  sourcePath: string;
  durationMs: number;
  silenceConfig?: SilenceConfig;
  editPlan: EditPlan;
  onEditPlanChange: (plan: EditPlan) => void;
  onPreview: (startMs: number, endMs: number) => void;
  analysisCandidates?: NonSpeechCandidate[];
}

export function ReviewControls({
  mediaId,
  sourcePath,
  durationMs,
  silenceConfig = DEFAULT_SILENCE_CONFIG,
  editPlan,
  onEditPlanChange,
  onPreview,
  analysisCandidates = [],
}: ReviewControlsProps) {
  const [suggestions, setSuggestions] = useState<CutSuggestion[]>([]);
  const [candidates, setCandidates] = useState<NonSpeechCandidate[]>(analysisCandidates);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRunAnalysis = async () => {
    setLoading(true);
    setError(null);
    try {
      const fusion = await analyzeNonSpeech({ sourcePath, durationMs, silenceConfig });
      setCandidates(fusion.candidates);
      const result: CutSuggestion[] = await invoke("generate_cut_suggestions", {
        sourceMediaId: mediaId,
        candidates: fusion.candidates,
        analysisVersion: fusion.analysis_version,
        existingPlan: editPlan,
        mediaDurationMs: durationMs,
      });
      setSuggestions(result);
      const generatedActions = result.map((suggestion) => suggestion.action);
      const generatedIds = new Set(generatedActions.map((action) => action.id));
      onEditPlanChange({
        ...editPlan,
        actions: [
          ...editPlan.actions.filter((action) => !generatedIds.has(action.id)),
          ...generatedActions,
        ],
      });
    } catch (e) {
      console.error(e);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleToggle = (id: string, enabled: boolean) => {
    setSuggestions((prev) =>
      prev.map((s) => (s.action.id === id ? { ...s, action: { ...s.action, enabled } } : s)),
    );
    onEditPlanChange({
      ...editPlan,
      actions: editPlan.actions.map((action) =>
        action.id === id ? { ...action, enabled, updatedAt: Date.now() } : action,
      ),
    });
  };

  const handleRemoveAll = () => {
    setSuggestions((prev) => prev.map((s) => ({ ...s, action: { ...s.action, enabled: false } })));
    onEditPlanChange({
      ...editPlan,
      actions: editPlan.actions.map((action) =>
        action.source === "local" && action.type === "cut"
          ? { ...action, enabled: false, updatedAt: Date.now() }
          : action,
      ),
    });
  };

  const handleKeepAll = () => {
    setSuggestions((prev) => prev.map((s) => ({ ...s, action: { ...s.action, enabled: true } })));
    onEditPlanChange({
      ...editPlan,
      actions: editPlan.actions.map((action) =>
        action.source === "local" && action.type === "cut"
          ? { ...action, enabled: true, updatedAt: Date.now() }
          : action,
      ),
    });
  };

  return (
    <div className="flex h-full flex-col rounded-lg border bg-card p-4">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-xl font-bold">Review Suggestions</h2>
        <Button onClick={handleRunAnalysis} disabled={loading}>
          {loading ? "Analyzing..." : "Generate Analysis"}
        </Button>
      </div>

      {suggestions.length > 0 && (
        <div className="mb-4 flex gap-2">
          <Button variant="secondary" size="sm" onClick={handleRemoveAll}>
            Keep All (No Cuts)
          </Button>
          <Button variant="secondary" size="sm" onClick={handleKeepAll}>
            Apply All Cuts
          </Button>
        </div>
      )}

      <ScrollArea className="flex-1 pr-4">
        {suggestions.length === 0 && !loading && (
          <div className="rounded-lg border border-dashed p-8 text-center text-muted-foreground">
            {candidates.length === 0
              ? "Chưa có Non-Speech Analysis cho media này. Hãy chạy local analysis trước."
              : 'Nhấn "Generate Analysis" để tạo đề xuất.'}
          </div>
        )}
        {error && (
          <p className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
            {error}
          </p>
        )}
        {suggestions.map((s) => (
          <SuggestionCard
            key={s.action.id}
            suggestion={s}
            onToggle={handleToggle}
            onPreview={onPreview}
          />
        ))}
      </ScrollArea>
    </div>
  );
}
