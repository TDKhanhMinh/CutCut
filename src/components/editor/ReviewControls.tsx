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
                { start_ms: 2000, end_ms: 3000, reason: "silence", evidence: { has_amplitude_silence: true, has_vad_non_speech: true, original_silence_duration_ms: 1000, original_vad_non_speech_duration_ms: 1000 }, confidence: "High", recommended_padding_ms: 0 },
                { start_ms: 6000, end_ms: 8000, reason: "noise_only", evidence: { has_amplitude_silence: false, has_vad_non_speech: true, original_silence_duration_ms: null, original_vad_non_speech_duration_ms: 2000 }, confidence: "Medium", recommended_padding_ms: 0 },
                { start_ms: 12000, end_ms: 12500, reason: "uncertain", evidence: { has_amplitude_silence: true, has_vad_non_speech: false, original_silence_duration_ms: 500, original_vad_non_speech_duration_ms: null }, confidence: "Low", recommended_padding_ms: 0 }
            ];

            const result: CutSuggestion[] = await invoke("generate_cut_suggestions", {
                sourceMediaId: mediaId,
                candidates: dummyCandidates,
                analysisVersion: "v1",
                existingTimeline: null
            });
            setSuggestions(result);
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    };

    const handleToggle = (id: string, enabled: boolean) => {
        setSuggestions(prev => prev.map(s => s.id === id ? { ...s, is_enabled: enabled } : s));
    };

    const handleRemoveAll = () => {
        setSuggestions(prev => prev.map(s => ({ ...s, is_enabled: false })));
    };

    const handleKeepAll = () => {
        setSuggestions(prev => prev.map(s => ({ ...s, is_enabled: true })));
    };

    return (
        <div className="flex flex-col h-full border rounded-lg bg-card p-4">
            <div className="flex items-center justify-between mb-4">
                <h2 className="text-xl font-bold">Review Suggestions</h2>
                <Button onClick={handleRunAnalysis} disabled={loading}>
                    {loading ? "Analyzing..." : "Generate Analysis"}
                </Button>
            </div>

            {suggestions.length > 0 && (
                <div className="flex gap-2 mb-4">
                    <Button variant="secondary" size="sm" onClick={handleRemoveAll}>Remove All Cuts</Button>
                    <Button variant="secondary" size="sm" onClick={handleKeepAll}>Keep All (No Cuts)</Button>
                </div>
            )}

            <ScrollArea className="flex-1 pr-4">
                {suggestions.length === 0 && !loading && (
                    <div className="text-muted-foreground text-center p-8 border border-dashed rounded-lg">
                        Click "Generate Analysis" to find removable segments.
                    </div>
                )}
                {suggestions.map(s => (
                    <SuggestionCard 
                        key={s.id} 
                        suggestion={s} 
                        startMs={s.action.start_ms || 0} // Safely get timestamps
                        endMs={s.action.end_ms || 0}
                        onToggle={handleToggle}
                        onPreview={onPreview}
                    />
                ))}
            </ScrollArea>
        </div>
    );
}
