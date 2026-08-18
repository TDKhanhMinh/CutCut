import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Play } from "lucide-react";

export interface CutSuggestion {
    id: string;
    source_media_id: string;
    action: any;
    confidence: "High" | "Medium" | "Low";
    reason: string;
    evidence: any;
    is_enabled: boolean;
    source_version: string;
}

interface SuggestionCardProps {
    suggestion: CutSuggestion;
    startMs: number;
    endMs: number;
    onToggle: (id: string, enabled: boolean) => void;
    onPreview: (startMs: number, endMs: number) => void;
}

export function SuggestionCard({ suggestion, startMs, endMs, onToggle, onPreview }: SuggestionCardProps) {
    const durationSec = ((endMs - startMs) / 1000).toFixed(1);
    
    // Map reason to friendly badge color
    const getBadgeVariant = (reason: string) => {
        if (reason === "silence") return "default";
        if (reason === "noise_only") return "secondary";
        return "destructive";
    };

    const getReasonLabel = (reason: string) => {
        if (reason === "silence") return "Silence";
        if (reason === "noise_only") return "Background Noise";
        if (reason === "uncertain") return "Uncertain Speech";
        return reason;
    };

    return (
        <Card className="mb-2">
            <CardContent className="p-4 flex items-center justify-between">
                <div className="flex flex-col gap-1">
                    <div className="flex items-center gap-2">
                        <span className="font-semibold">{durationSec}s Cut</span>
                        <Badge variant={getBadgeVariant(suggestion.reason)}>
                            {getReasonLabel(suggestion.reason)}
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
                        <span className="text-sm">{suggestion.is_enabled ? "Remove" : "Keep"}</span>
                        <Switch 
                            checked={!suggestion.is_enabled} 
                            onCheckedChange={(checked) => onToggle(suggestion.id, !checked)} 
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
