import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Play } from "lucide-react";
import { EditAction } from "@/types/project";
import { DetectorEvidence } from "@/types/fusion";

export interface CutSuggestion {
  action: EditAction;
  evidence: DetectorEvidence;
  source_version: string;
}

interface SuggestionCardProps {
  suggestion: CutSuggestion;
  onToggle: (id: string, enabled: boolean) => void;
  onPreview: (startMs: number, endMs: number) => void;
}

export function SuggestionCard({ suggestion, onToggle, onPreview }: SuggestionCardProps) {
  const startMs = suggestion.action.startMs;
  const endMs = suggestion.action.endMs;
  const durationSec = ((endMs - startMs) / 1000).toFixed(1);

  // Map reason to friendly badge color
  const getBadgeVariant = (reason: string) => {
    if (reason === "silence") return "default" as const;
    if (reason === "noise_only") return "secondary" as const;
    return "destructive" as const;
  };

  const getReasonLabel = (reason: string) => {
    if (reason === "silence") return "Silence";
    if (reason === "noise_only") return "Background Noise";
    if (reason === "uncertain") return "Uncertain Speech";
    return reason;
  };

  return (
    <Card className="mb-2">
      <CardContent className="flex items-center justify-between p-4">
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <span className="font-semibold">{durationSec}s Cut</span>
            <Badge variant={getBadgeVariant(suggestion.action.reason)}>
              {getReasonLabel(suggestion.action.reason)}
            </Badge>
          </div>
          <span className="text-sm text-muted-foreground">
            {formatTime(startMs)} - {formatTime(endMs)}
          </span>
        </div>

        <div className="flex items-center gap-4">
          <Button variant="outline" size="icon" onClick={() => onPreview(startMs, endMs)}>
            <Play className="h-4 w-4" />
          </Button>
          <div className="flex items-center gap-2">
            <span className="text-sm">{suggestion.action.enabled ? "Remove" : "Keep"}</span>
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
