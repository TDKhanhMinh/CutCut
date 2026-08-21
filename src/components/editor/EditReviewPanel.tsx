import { useMemo, useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { useProjectStore } from "@/stores/useProjectStore";
import { ActionCard } from "./ActionCard";

interface EditReviewPanelProps {
  mediaId: string;
  onPreview: (startMs: number, endMs: number) => void;
}

type FilterSource = "all" | "local" | "ai" | "user";

export function EditReviewPanel({ mediaId, onPreview }: EditReviewPanelProps) {
  const { activeProject, updateProject } = useProjectStore();
  const [loading, setLoading] = useState(false);
  const [filterSource, setFilterSource] = useState<FilterSource>("all");
  const actions = activeProject?.editPlan.actions;

  const displayActions = useMemo(() => {
    const allActions = actions ?? [];
    const filtered = filterSource === "all" ? allActions : allActions.filter((action) => action.source === filterSource);
    return [...filtered].sort((a, b) => a.startMs - b.startMs);
  }, [actions, filterSource]);

  const handleRunAnalysis = async () => {
    setLoading(true);
    try {
      updateProject((draft) => {
        const now = Date.now();
        draft.editPlan.actions.push(
          {
            id: crypto.randomUUID(), type: "cut", sourceMediaId: mediaId, startMs: 2_000, endMs: 3_000,
            source: "local", reason: "silence", confidence: null, enabled: true, createdAt: now, updatedAt: now,
          },
          {
            id: crypto.randomUUID(), type: "cut", sourceMediaId: mediaId, startMs: 12_000, endMs: 12_500,
            source: "ai", reason: "false_start", confidence: 0.85, enabled: true, createdAt: now, updatedAt: now,
          },
        );
      });
    } finally {
      setLoading(false);
    }
  };

  const setEnabledFor = (ids: Set<string>, enabled: boolean) => {
    updateProject((draft) => {
      draft.editPlan.actions.forEach((action) => {
        if (ids.has(action.id)) {
          action.enabled = enabled;
          action.isManualModified = true;
        }
      });
    });
  };

  const visibleCutIds = new Set(displayActions.filter((action) => action.type === "cut").map((action) => action.id));

  return (
    <div className="flex h-full flex-col rounded-lg border bg-card p-4">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-xl font-bold">Review Suggestions</h2>
        <Button onClick={handleRunAnalysis} disabled={loading}>{loading ? "Analyzing..." : "Generate Analysis"}</Button>
      </div>
      {(actions?.length ?? 0) > 0 && (
        <div className="mb-4 flex flex-col gap-2">
          <div className="flex gap-2">
            <Button variant="secondary" size="sm" onClick={() => setEnabledFor(visibleCutIds, false)}>Remove All Cuts</Button>
            <Button variant="secondary" size="sm" onClick={() => setEnabledFor(visibleCutIds, true)}>Keep All (No Cuts)</Button>
          </div>
          <div className="flex gap-1 rounded-md bg-secondary/50 p-1">
            {(["all", "local", "ai", "user"] as const).map((source) => (
              <Button key={source} variant={filterSource === source ? "secondary" : "ghost"} size="sm" onClick={() => setFilterSource(source)}>
                {source === "all" ? "All" : source === "ai" ? "AI Only" : source[0].toUpperCase() + source.slice(1)}
              </Button>
            ))}
          </div>
        </div>
      )}
      <ScrollArea className="flex-1 pr-4">
        {displayActions.length === 0 && !loading && (
          <div className="rounded-lg border border-dashed p-8 text-center text-muted-foreground">
            {(actions?.length ?? 0) === 0 ? "Click 'Generate Analysis' to find removable segments." : "No actions match the selected filter."}
          </div>
        )}
        {displayActions.map((action) => (
          <ActionCard key={action.id} action={action} onToggle={(id, enabled) => {
            updateProject((draft) => {
              const candidate = draft.editPlan.actions.find((item) => item.id === id);
              if (candidate) {
                candidate.enabled = enabled;
                candidate.isManualModified = true;
              }
            });
          }} onPreview={onPreview} />
        ))}
      </ScrollArea>
    </div>
  );
}
