import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Play } from "lucide-react";
import { EditAction } from "@/types/project";
import { DetectorEvidence } from "@/types/fusion";
import { useI18n } from "@/i18n";

export interface CutSuggestion {
  action: EditAction;
  evidence: DetectorEvidence;
  sourceVersion: string;
  kind?: "silence" | "filler";
  reviewRequired?: boolean;
}

interface SuggestionCardProps {
  suggestion: CutSuggestion;
  onToggle: (id: string, enabled: boolean) => void;
  onPreview: (startMs: number, endMs: number) => void;
}

export function SuggestionCard({ suggestion, onToggle, onPreview }: SuggestionCardProps) {
  const { t } = useI18n();
  const startMs = suggestion.action.startMs;
  const endMs = suggestion.action.endMs;
  const durationSec = ((endMs - startMs) / 1000).toFixed(1);

  // Map reason to friendly badge color
  const getBadgeVariant = (reason: string) => {
    if (reason.startsWith("filler:")) return "secondary" as const;
    if (reason === "silence") return "default" as const;
    if (reason === "noise_only") return "secondary" as const;
    return "destructive" as const;
  };

  const getReasonLabel = (reason: string) => {
    if (reason === "silence") return t("editor.silence");
    if (reason === "noise_only") return t("editor.backgroundNoise");
    if (reason === "uncertain") return t("editor.uncertainSpeech");
    if (reason.startsWith("filler:"))
      return `${t("editor.filler")}: ${reason.slice("filler:".length)}`;
    return reason;
  };

  return (
    <Card className="mb-2">
      <CardContent className="flex items-center justify-between p-4">
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <span className="font-semibold">
              {durationSec}s {t("editor.cut")}
            </span>
            <Badge variant={getBadgeVariant(suggestion.action.reason)}>
              {getReasonLabel(suggestion.action.reason)}
            </Badge>
          </div>
          <span className="text-sm text-muted-foreground">
            {formatTime(startMs)} - {formatTime(endMs)}
          </span>
          {suggestion.reviewRequired && (
            <span className="text-xs text-amber-600">{t("editor.reviewRequired")}</span>
          )}
        </div>

        <div className="flex items-center gap-4">
          <Button
            variant="outline"
            size="icon"
            aria-label={t("editor.previewRange", {
              start: formatTime(startMs),
              end: formatTime(endMs),
            })}
            onClick={() => onPreview(startMs, endMs)}
          >
            <Play className="h-4 w-4" />
          </Button>
          <div className="flex items-center gap-2">
            <span className="text-sm">
              {suggestion.action.enabled ? t("editor.remove") : t("editor.keep")}
            </span>
            <Switch
              checked={!suggestion.action.enabled}
              onCheckedChange={(checked) => onToggle(suggestion.action.id, !checked)}
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function formatTime(ms: number) {
  const totalSeconds = Math.floor(ms / 1000);
  const m = Math.floor(totalSeconds / 60);
  const s = totalSeconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}
