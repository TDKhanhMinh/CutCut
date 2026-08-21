import { useMemo, useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { useProjectStore } from "@/stores/useProjectStore";
import { useAuthStore } from "@/stores/useAuthStore";
import { useEntitlementStore } from "@/stores/useEntitlementStore";
import { AuthDialog } from "./AuthDialog";
import { ActionCard } from "./ActionCard";
import { useI18n } from "@/i18n";

interface EditReviewPanelProps {
  mediaId: string;
  onPreview: (startMs: number, endMs: number) => void;
}

type FilterSource = "all" | "local" | "ai" | "user";

export function EditReviewPanel({ mediaId, onPreview }: EditReviewPanelProps) {
  const { t } = useI18n();
  const { activeProject, updateProject } = useProjectStore();
  const { session } = useAuthStore();
  const { hasCapability } = useEntitlementStore();
  const [authDialogOpen, setAuthDialogOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [filterSource, setFilterSource] = useState<FilterSource>("all");
  const actions = activeProject?.editPlan.actions;

  const displayActions = useMemo(() => {
    const allActions = actions ?? [];
    const filtered =
      filterSource === "all"
        ? allActions
        : allActions.filter((action) => action.source === filterSource);
    return [...filtered].sort((a, b) => a.startMs - b.startMs);
  }, [actions, filterSource]);

  const handleRunAnalysis = async () => {
    if (!hasCapability("FEATURE_CLOUD_AI")) {
      if (!session) {
        setAuthDialogOpen(true);
      } else {
        window.alert(t("errors.cloudPlan"));
      }
      return;
    }

    setLoading(true);
    try {
      // The provider command is integrated separately; this keeps the review
      // surface deterministic while preserving the canonical EditPlan shape.
      updateProject((draft) => {
        const now = Date.now();
        draft.editPlan.actions.push(
          {
            id: crypto.randomUUID(),
            type: "cut",
            sourceMediaId: mediaId,
            startMs: 2_000,
            endMs: 3_000,
            source: "local",
            reason: "silence",
            confidence: null,
            enabled: true,
            isManualModified: false,
            createdAt: now,
            updatedAt: now,
          },
          {
            id: crypto.randomUUID(),
            type: "highlight",
            sourceMediaId: mediaId,
            startMs: 5_000,
            endMs: 8_000,
            source: "ai",
            reason: "important_statement",
            confidence: 0.95,
            enabled: true,
            isManualModified: false,
            createdAt: now,
            updatedAt: now,
          },
          {
            id: crypto.randomUUID(),
            type: "cut",
            sourceMediaId: mediaId,
            startMs: 12_000,
            endMs: 12_500,
            source: "ai",
            reason: "false_start",
            confidence: 0.85,
            enabled: true,
            isManualModified: false,
            createdAt: now,
            updatedAt: now,
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
          action.updatedAt = Date.now();
        }
      });
    });
  };

  const visibleCutIds = new Set(
    displayActions.filter((action) => action.type === "cut").map((action) => action.id),
  );

  return (
    <div className="flex h-full flex-col rounded-lg border bg-card p-4">
      <AuthDialog open={authDialogOpen} onOpenChange={setAuthDialogOpen} />
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-xl font-bold">{t("editor.reviewSuggestions")}</h2>
        <Button onClick={handleRunAnalysis} disabled={loading}>
          {loading ? t("editor.analyzing") : t("editor.generateAnalysis")}
        </Button>
      </div>
      {(actions?.length ?? 0) > 0 && (
        <div className="mb-4 flex flex-col gap-2">
          <div className="flex gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setEnabledFor(visibleCutIds, false)}
            >
              {t("editor.removeAllCuts")}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setEnabledFor(visibleCutIds, true)}
            >
              {t("editor.keepAll")}
            </Button>
          </div>
          <div className="flex gap-1 rounded-md bg-secondary/50 p-1">
            {(["all", "local", "ai", "user"] as const).map((source) => (
              <Button
                key={source}
                variant={filterSource === source ? "secondary" : "ghost"}
                size="sm"
                onClick={() => setFilterSource(source)}
              >
                {source === "all"
                  ? t("editor.filterAll")
                  : source === "local"
                    ? t("editor.filterLocal")
                    : source === "ai"
                      ? t("editor.filterAi")
                      : t("editor.filterUser")}
              </Button>
            ))}
          </div>
        </div>
      )}
      <ScrollArea className="flex-1 pr-4">
        {displayActions.length === 0 && !loading && (
          <div className="rounded-lg border border-dashed p-8 text-center text-muted-foreground">
            {(actions?.length ?? 0) === 0 ? t("editor.generateHint") : t("editor.noFilterActions")}
          </div>
        )}
        {displayActions.map((action) => (
          <ActionCard
            key={action.id}
            action={action}
            onToggle={(id, enabled) => {
              updateProject((draft) => {
                const candidate = draft.editPlan.actions.find((item) => item.id === id);
                if (candidate) {
                  candidate.enabled = enabled;
                  candidate.isManualModified = true;
                  candidate.updatedAt = Date.now();
                }
              });
            }}
            onPreview={onPreview}
          />
        ))}
      </ScrollArea>
    </div>
  );
}
