import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Play } from "lucide-react";
import { EditAction } from "@/types/project";

interface ActionCardProps {
    action: EditAction;
    onToggle: (id: string, enabled: boolean) => void;
    onPreview: (startMs: number, endMs: number) => void;
}

export function ActionCard({ action, onToggle, onPreview }: ActionCardProps) {
    // TypeScript safe payload cast since we only pass Cut or Highlight here
    const startMs = 'startMs' in action.payload ? action.payload.startMs : 0;
    const endMs = 'endMs' in action.payload ? action.payload.endMs : 0;
    const durationSec = ((endMs - startMs) / 1000).toFixed(1);
    
    // UI Helpers
    const isCut = action.payload.type === "cut";
    const isHighlight = action.payload.type === "highlight";
    
    const getBadgeVariant = () => {
        if (isHighlight) return "default";
        if (action.source === "aiAgent") return "destructive"; // AI Cuts are destructive suggestions
        if (action.reason.includes("silence")) return "secondary";
        return "outline";
    };

    const getSourceLabel = () => {
        switch (action.source) {
            case "aiAgent": return "AI";
            case "localDetector": return "Local";
            case "userManual": return "User";
            default: return action.source;
        }
    };

    const getReasonLabel = () => {
        if (action.reason.includes("false_start")) return "False Start";
        if (action.reason.includes("repeated_take")) return "Repeated Take";
        if (action.reason.includes("redundant_sentence")) return "Redundant";
        if (action.reason.includes("important_statement")) return "Highlight";
        if (action.reason.includes("silence")) return "Silence";
        if (action.reason.includes("noise_only")) return "Background Noise";
        return action.reason;
    };

    const actionTitle = isHighlight ? "Highlight" : (isCut ? "Cut" : "Action");

    return (
        <Card className="mb-2">
            <CardContent className="p-4 flex items-center justify-between">
                <div className="flex flex-col gap-1">
                    <div className="flex items-center gap-2">
                        <span className="font-semibold">{durationSec}s {actionTitle}</span>
                        <Badge variant={getBadgeVariant()}>
                            {getReasonLabel()}
                        </Badge>
                        <Badge variant="outline" className="text-xs uppercase">
                            {getSourceLabel()}
                        </Badge>
                        {action.confidence && action.source === "aiAgent" && (
                            <span className="text-xs text-muted-foreground ml-1">
                                {(parseFloat(action.confidence) * 100).toFixed(0)}% confident
                            </span>
                        )}
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
                        <span className="text-sm">{action.enabled ? (isCut ? "Remove" : "Keep") : "Skip"}</span>
                        <Switch 
                            checked={!action.enabled} // Checked implies we are cutting/skipping
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
    const m = Math.floor(totalSeconds / 60);
    const s = totalSeconds % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
}
