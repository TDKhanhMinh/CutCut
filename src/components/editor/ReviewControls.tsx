import { useState } from "react";
import { CutSuggestion, SuggestionCard } from "./SuggestionCard";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";

interface ReviewControlsProps {
  mediaId: string;
  onPreview: (startMs: number, endMs: number) => void;
}

export function ReviewControls({ mediaId, onPreview }: ReviewControlsProps) {
  const [suggestions, setSuggestions] = useState<CutSuggestion[]>([]);
  const [loading, setLoading] = useState(false);

  const handleRunAnalysis = async () => {
    setLoading(true);
    try {
      // Simulated dummy candidates for demonstration before VAD is fully linked in UI
      const dummyCandidates = [
        {
          start_ms: 2000,
          end_ms: 3000,
          reason: "silence",
          evidence: {
            has_amplitude_silence: true,
            has_vad_non_speech: true,
            original_silence_duration_ms: 1000,
            original_vad_non_speech_duration_ms: 1000,
          },
          confidence: "High",
          recommended_padding_ms: 0,
        },
        {
          start_ms: 6000,
          end_ms: 8000,
          reason: "noise_only",
          evidence: {
            has_amplitude_silence: false,
            has_vad_non_speech: true,
            original_silence_duration_ms: null,
            original_vad_non_speech_duration_ms: 2000,
          },
          confidence: "Medium",
          recommended_padding_ms: 0,
        },
        {
          start_ms: 12000,
          end_ms: 12500,
          reason: "uncertain",
          evidence: {
            has_amplitude_silence: true,
            has_vad_non_speech: false,
            original_silence_duration_ms: 500,
            original_vad_non_speech_duration_ms: null,
          },
          confidence: "Low",
          recommended_padding_ms: 0,
        },
      ];

      const result: CutSuggestion[] = await invoke("generate_cut_suggestions", {
        sourceMediaId: mediaId,
        candidates: dummyCandidates,
        analysisVersion: "v1",
        existingPlan: null,
      });
      setSuggestions(result);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const handleToggle = (id: string, enabled: boolean) => {
    setSuggestions((prev) =>
      prev.map((s) => (s.action.id === id ? { ...s, action: { ...s.action, enabled } } : s)),
    );
  };

  const handleRemoveAll = () => {
    setSuggestions((prev) => prev.map((s) => ({ ...s, action: { ...s.action, enabled: false } })));
  };

  const handleKeepAll = () => {
    setSuggestions((prev) => prev.map((s) => ({ ...s, action: { ...s.action, enabled: true } })));
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
            Remove All Cuts
          </Button>
          <Button variant="secondary" size="sm" onClick={handleKeepAll}>
            Keep All (No Cuts)
          </Button>
        </div>
      )}

      <ScrollArea className="flex-1 pr-4">
        {suggestions.length === 0 && !loading && (
          <div className="rounded-lg border border-dashed p-8 text-center text-muted-foreground">
            Click "Generate Analysis" to find removable segments.
          </div>
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
