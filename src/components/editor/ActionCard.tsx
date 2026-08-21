import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Play } from "lucide-react";
import type { EditAction } from "@/types/project";
import { useI18n } from "@/i18n";

interface ActionCardProps {
  action: EditAction;
  onToggle: (id: string, enabled: boolean) => void;
  onPreview: (startMs: number, endMs: number) => void;
}

export function ActionCard({ action, onToggle, onPreview }: ActionCardProps) {
  const { t } = useI18n();
  const durationSec = ((action.endMs - action.startMs) / 1000).toFixed(1);
  const isCut = action.type === "cut";
  const isHighlight = action.type === "highlight";
  const getBadgeVariant = () => {
    if (action.source === "ai") return "destructive" as const;
    if (action.reason.includes("silence")) return "secondary" as const;
    return "outline" as const;
  };
  const sourceLabel = action.source === "ai" ? "AI" : action.source === "local" ? "Local" : "User";
  const reasonLabel = action.reason
    .replace("false_start", t("editor.falseStart"))
    .replace("repeated_take", t("editor.repeatedTake"))
    .replace("redundant_sentence", t("editor.redundant"))
    .replace("important_statement", t("editor.highlight"))
    .replace("noise_only", t("editor.backgroundNoise"));

  return (
    <Card className="mb-2">
      <CardContent className="flex items-center justify-between p-4">
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <span className="font-semibold">
              {durationSec}s{" "}
              {isCut ? t("editor.cut") : isHighlight ? t("editor.highlight") : t("editor.action")}
            </span>
            <Badge variant={getBadgeVariant()}>{reasonLabel}</Badge>
            <Badge variant="outline" className="text-xs uppercase">
              {sourceLabel}
              {action.isManualModified ? " (Mod)" : ""}
            </Badge>
            {action.confidence !== null && action.source === "ai" && (
              <span className="ml-1 text-xs text-muted-foreground">
                {(action.confidence * 100).toFixed(0)}
                {t("editor.confident")}
              </span>
            )}
          </div>
          <span className="text-sm text-muted-foreground">
            {formatTime(action.startMs)} - {formatTime(action.endMs)}
          </span>
        </div>
        <div className="flex items-center gap-4">
          <Button
            variant="outline"
            size="icon"
            onClick={() => onPreview(action.startMs, action.endMs)}
          >
            <Play className="h-4 w-4" />
          </Button>
          <div className="flex items-center gap-2">
            <span className="text-sm">
              {action.enabled ? (isCut ? t("editor.remove") : t("editor.keep")) : t("editor.skip")}
            </span>
            <Switch
              checked={!action.enabled}
              onCheckedChange={(checked) => onToggle(action.id, !checked)}
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function formatTime(ms: number) {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  return `${minutes}:${(totalSeconds % 60).toString().padStart(2, "0")}`;
}
