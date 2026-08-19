import { useState, useMemo } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
// Removed ToggleGroup
// Removed invoke
import { useProjectStore } from "@/stores/useProjectStore";
import { ActionCard } from "./ActionCard";

interface EditReviewPanelProps {
    mediaId: string;
    onPreview: (startMs: number, endMs: number) => void;
}

type FilterSource = "all" | "localDetector" | "aiAgent" | "userManual";

export function EditReviewPanel({ mediaId, onPreview }: EditReviewPanelProps) {
    const { activeProject, updateProject } = useProjectStore();
    const [loading, setLoading] = useState(false);
    const [filterSource, setFilterSource] = useState<FilterSource>("all");

    const actions = activeProject?.editPlan.actions || [];

    const displayActions = useMemo(() => {
        // Filter by source
        let filtered = actions;
        if (filterSource !== "all") {
            filtered = actions.filter(a => a.source === filterSource);
        }

        // Sort by startMs
        return [...filtered].sort((a, b) => {
            const startA = 'startMs' in a.payload ? a.payload.startMs as number : 0;
            const startB = 'startMs' in b.payload ? b.payload.startMs as number : 0;
            return startA - startB;
        });
    }, [actions, filterSource]);

    const handleRunAnalysis = async () => {
        setLoading(true);
        try {
            // TODO: In Task 38/39, we will call the real AI merger backend command
            // For now, we mock some actions directly into the store to test UI
            updateProject(draft => {
                if (!draft) return;
                draft.editPlan.actions.push({
                    id: crypto.randomUUID(),
                    sourceMediaId: mediaId,
                    payload: { type: "cut", startMs: 2000, endMs: 3000 },
                    source: "localDetector",
                    reason: "silence",
                    enabled: true,
                    createdAt: Date.now(),
                    updatedAt: Date.now()
                });
                draft.editPlan.actions.push({
                    id: crypto.randomUUID(),
                    sourceMediaId: mediaId,
                    payload: { type: "highlight", startMs: 5000, endMs: 8000 },
                    source: "aiAgent",
                    reason: "important_statement (CTA)",
                    confidence: "0.95",
                    enabled: true,
                    createdAt: Date.now(),
                    updatedAt: Date.now()
                });
                draft.editPlan.actions.push({
                    id: crypto.randomUUID(),
                    sourceMediaId: mediaId,
                    payload: { type: "cut", startMs: 12000, endMs: 12500 },
                    source: "aiAgent",
                    reason: "false_start",
                    confidence: "0.85",
                    enabled: true,
                    createdAt: Date.now(),
                    updatedAt: Date.now()
                });
            });
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    };

    const handleToggle = (id: string, enabled: boolean) => {
        updateProject(draft => {
            if (!draft) return;
            const action = draft.editPlan.actions.find(a => a.id === id);
            if (action) {
                action.enabled = enabled;
            }
        });
    };

    const handleRemoveAll = () => {
        updateProject(draft => {
            if (!draft) return;
            draft.editPlan.actions.forEach(a => { if (a.payload.type === "cut") a.enabled = false; });
        });
    };

    const handleKeepAll = () => {
        updateProject(draft => {
            if (!draft) return;
            draft.editPlan.actions.forEach(a => { if (a.payload.type === "cut") a.enabled = true; });
        });
    };

    return (
        <div className="flex flex-col h-full border rounded-lg bg-card p-4">
            <div className="flex items-center justify-between mb-4">
                <h2 className="text-xl font-bold">Review Suggestions</h2>
                <Button onClick={handleRunAnalysis} disabled={loading}>
                    {loading ? "Analyzing..." : "Generate Analysis"}
                </Button>
            </div>

            {actions.length > 0 && (
                <div className="flex flex-col gap-2 mb-4">
                    <div className="flex gap-2">
                        <Button variant="secondary" size="sm" onClick={handleRemoveAll}>Remove All Cuts</Button>
                        <Button variant="secondary" size="sm" onClick={handleKeepAll}>Keep All (No Cuts)</Button>
                    </div>
                    
                    <div className="flex bg-secondary/50 p-1 rounded-md gap-1">
                        <Button variant={filterSource === "all" ? "secondary" : "ghost"} size="sm" onClick={() => setFilterSource("all")}>All</Button>
                        <Button variant={filterSource === "localDetector" ? "secondary" : "ghost"} size="sm" onClick={() => setFilterSource("localDetector")}>Local</Button>
                        <Button variant={filterSource === "aiAgent" ? "secondary" : "ghost"} size="sm" onClick={() => setFilterSource("aiAgent")}>AI Only</Button>
                        <Button variant={filterSource === "userManual" ? "secondary" : "ghost"} size="sm" onClick={() => setFilterSource("userManual")}>User</Button>
                    </div>
                </div>
            )}

            <ScrollArea className="flex-1 pr-4">
                {displayActions.length === 0 && !loading && (
                    <div className="text-muted-foreground text-center p-8 border border-dashed rounded-lg">
                        {actions.length === 0 ? "Click 'Generate Analysis' to find removable segments." : "No actions match the selected filter."}
                    </div>
                )}
                {displayActions.map(action => (
                    <ActionCard 
                        key={action.id} 
                        action={action} 
                        onToggle={handleToggle}
                        onPreview={onPreview}
                    />
                ))}
            </ScrollArea>
        </div>
    );
}
