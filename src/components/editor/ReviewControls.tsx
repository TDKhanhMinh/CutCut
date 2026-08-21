import { useMemo, useState } from "react";
import { CutSuggestion, SuggestionCard } from "./SuggestionCard";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import type { EditPlan } from "@/types/project";
import type { MediaSource, Transcript } from "@/types/project";
import type { NonSpeechCandidate } from "@/types/fusion";
import { DEFAULT_SILENCE_CONFIG, type SilenceConfig } from "@/types/silence";
import { analyzeNonSpeech } from "@/services/nonSpeechAnalysis";
import { detectFillerCandidates } from "@/services/filler";
import { validateEditPlan } from "@/services/project";
import { analyzeAndMerge } from "@/services/ai";
import { useAIConfigStore } from "@/stores/useAIConfigStore";
import { useI18n } from "@/i18n";

interface ReviewControlsProps {
  mediaId: string;
  media: MediaSource[];
  sourcePath: string;
  durationMs: number;
  transcript?: Transcript | null;
  silenceConfig?: SilenceConfig;
  editPlan: EditPlan;
  onEditPlanChange: (plan: EditPlan) => void;
  onPreview: (startMs: number, endMs: number) => void;
  analysisCandidates?: NonSpeechCandidate[];
}

export function ReviewControls({
  mediaId,
  media,
  sourcePath,
  durationMs,
  transcript,
  silenceConfig = DEFAULT_SILENCE_CONFIG,
  editPlan,
  onEditPlanChange,
  onPreview,
  analysisCandidates = [],
}: ReviewControlsProps) {
  const { t } = useI18n();
  const persistedSuggestions = useMemo(
    () =>
      editPlan.actions
        .filter(
          (action) =>
            action.source === "local" &&
            action.type === "cut" &&
            action.sourceMediaId === mediaId &&
            (action.reason === "silence" ||
              action.reason === "noise_only" ||
              action.reason === "uncertain" ||
              action.reason.startsWith("filler:")),
        )
        .map((action) => ({
          action,
          evidence: {
            has_amplitude_silence: action.reason === "silence",
            has_vad_non_speech: action.reason === "noise_only",
          },
          sourceVersion: "persisted-edit-plan",
          kind: action.reason.startsWith("filler:") ? ("filler" as const) : ("silence" as const),
          reviewRequired: action.reason.startsWith("filler:"),
        })),
    [editPlan.actions, mediaId],
  );
  const [metadataById, setMetadataById] = useState<
    Record<string, Pick<CutSuggestion, "evidence" | "sourceVersion" | "kind" | "reviewRequired">>
  >({});
  const suggestions = useMemo(
    () =>
      persistedSuggestions.map((suggestion) => ({
        ...suggestion,
        ...metadataById[suggestion.action.id],
      })),
    [metadataById, persistedSuggestions],
  );
  const [candidates, setCandidates] = useState<NonSpeechCandidate[]>(analysisCandidates);
  const [loading, setLoading] = useState(false);
  const [aiLoading, setAiLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const aiMode = useAIConfigStore((state) => state.mode);

  const handleRunAnalysis = async () => {
    setLoading(true);
    setError(null);
    try {
      const fusionPromise = analyzeNonSpeech({ sourcePath, durationMs, silenceConfig });
      const fillerPromise = transcript
        ? detectFillerCandidates(mediaId, transcript, durationMs, 40).catch((fillerError) => {
            console.error("Filler analysis unavailable:", fillerError);
            return null;
          })
        : Promise.resolve(null);
      const [fusion, filler] = await Promise.all([fusionPromise, fillerPromise]);
      setCandidates(fusion.candidates);
      const silenceResult: CutSuggestion[] = await invoke("generate_cut_suggestions", {
        sourceMediaId: mediaId,
        candidates: fusion.candidates,
        analysisVersion: fusion.analysis_version,
        existingPlan: editPlan,
        mediaDurationMs: durationMs,
      });
      const fillerSuggestions: CutSuggestion[] =
        filler?.actions.map((action) => ({
          action,
          evidence: {
            has_amplitude_silence: false,
            has_vad_non_speech: false,
          },
          sourceVersion: `filler-dictionary-${filler.dictionaryVersion}`,
          kind: "filler",
          reviewRequired: filler.candidates.find((candidate) => candidate.id === action.id)
            ?.reviewRequired,
        })) ?? [];
      const result = [...silenceResult, ...fillerSuggestions];
      const generatedActions = result.map((suggestion) => {
        const previous = editPlan.actions.find((action) => action.id === suggestion.action.id);
        return previous
          ? { ...suggestion.action, enabled: previous.enabled, updatedAt: previous.updatedAt }
          : suggestion.action;
      });
      const generatedIds = new Set(generatedActions.map((action) => action.id));
      const nextPlan: EditPlan = {
        ...editPlan,
        actions: [
          ...editPlan.actions.filter((action) => !generatedIds.has(action.id)),
          ...generatedActions,
        ],
      };
      const validation = await validateEditPlan(nextPlan, media);
      const errors = validation.issues.filter((issue) => issue.level === "error");
      if (errors.length > 0) {
        throw new Error(errors.map((issue) => issue.message).join("; "));
      }
      setMetadataById((previous) => ({
        ...previous,
        ...Object.fromEntries(
          result.map((suggestion) => [
            suggestion.action.id,
            {
              evidence: suggestion.evidence,
              sourceVersion: suggestion.sourceVersion,
              kind: suggestion.kind,
              reviewRequired: suggestion.reviewRequired,
            },
          ]),
        ),
      }));
      onEditPlanChange(nextPlan);
    } catch (e) {
      console.error(e);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleToggle = (id: string, enabled: boolean) => {
    onEditPlanChange({
      ...editPlan,
      actions: editPlan.actions.map((action) =>
        action.id === id
          ? { ...action, enabled, isManualModified: true, updatedAt: Date.now() }
          : action,
      ),
    });
  };

  const handleRunAIAnalysis = async () => {
    if (!transcript) return;
    setAiLoading(true);
    setError(null);
    try {
      const nextPlan = await analyzeAndMerge(mediaId, transcript, editPlan, media);
      onEditPlanChange(nextPlan);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setAiLoading(false);
    }
  };

  const handleRemoveAll = () => {
    const suggestionIds = new Set(suggestions.map((suggestion) => suggestion.action.id));
    onEditPlanChange({
      ...editPlan,
      actions: editPlan.actions.map((action) =>
        suggestionIds.has(action.id)
          ? { ...action, enabled: false, isManualModified: true, updatedAt: Date.now() }
          : action,
      ),
    });
  };

  const handleKeepAll = () => {
    const suggestionIds = new Set(suggestions.map((suggestion) => suggestion.action.id));
    onEditPlanChange({
      ...editPlan,
      actions: editPlan.actions.map((action) =>
        suggestionIds.has(action.id)
          ? { ...action, enabled: true, isManualModified: true, updatedAt: Date.now() }
          : action,
      ),
    });
  };

  return (
    <div className="flex h-full flex-col rounded-lg border bg-card p-4">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-xl font-bold">{t("editor.reviewSuggestions")}</h2>
        <Button onClick={handleRunAnalysis} disabled={loading}>
          {loading ? t("editor.analyzing") : t("editor.generateAnalysis")}
        </Button>
        <Button
          onClick={() => void handleRunAIAnalysis()}
          disabled={aiLoading || !transcript}
          variant="secondary"
        >
          {aiLoading
            ? t("editor.aiAnalyzing")
            : aiMode === "byok"
              ? t("editor.analyzeByok")
              : t("editor.analyzeHosted")}
        </Button>
      </div>

      {suggestions.length > 0 && (
        <div className="mb-4 flex gap-2">
          <Button variant="secondary" size="sm" onClick={handleRemoveAll}>
            {t("editor.keepAll")}
          </Button>
          <Button variant="secondary" size="sm" onClick={handleKeepAll}>
            {t("editor.applyAll")}
          </Button>
        </div>
      )}

      <ScrollArea className="flex-1 pr-4">
        {suggestions.length === 0 && !loading && (
          <div className="rounded-lg border border-dashed p-8 text-center text-muted-foreground">
            {candidates.length === 0 && !transcript
              ? t("editor.noAnalysis")
              : t("editor.runAnalysisHint")}
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
