import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { VolumeX, Loader2, CheckCircle2, PlayCircle } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "../ui/dialog";
import { Button } from "../ui/button";
import { RadioGroup, RadioGroupItem } from "../ui/radio-group";
import { Label } from "../ui/label";
import { SilencePreset, SilenceConfig, SilenceSettings, SilenceInterval } from "@/types/silence";

interface SilenceSettingsPanelProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  config: SilenceConfig;
  onChange: (config: SilenceConfig) => void;
  testVideoPath?: string; // Path to run detection against
}

const PRESET_MAPPING: Record<Exclude<SilencePreset, "custom">, SilenceSettings> = {
  conservative: { thresholdDb: -40, minDurationMs: 1500 },
  balanced: { thresholdDb: -35, minDurationMs: 750 },
  aggressive: { thresholdDb: -30, minDurationMs: 400 },
};

export function SilenceSettingsPanel({
  isOpen,
  onOpenChange,
  config,
  onChange,
  testVideoPath,
}: SilenceSettingsPanelProps) {
  const [localConfig, setLocalConfig] = useState<SilenceConfig>(config);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ count: number; totalDurationMs: number } | null>(null);

  // Sync prop to local state when opening
  if (isOpen && localConfig !== config && !isTesting) {
    setLocalConfig(config);
  }

  const handlePresetChange = (preset: SilencePreset) => {
    if (preset === "custom") {
      setLocalConfig({ preset, settings: { ...localConfig.settings } });
    } else {
      setLocalConfig({ preset, settings: PRESET_MAPPING[preset] });
    }
  };

  const handleSave = () => {
    onChange(localConfig);
    onOpenChange(false);
  };

  const handleTest = async () => {
    if (!testVideoPath) {
      alert("No active video to test detection against. Please import a video first.");
      return;
    }
    try {
      setIsTesting(true);
      setTestResult(null);
      const jobId = `silence-test-${Date.now()}`;
      
      const unlisten = await listen("media-job", (event: { payload: { jobId: string; state: string; message?: string; error?: string } }) => {
        const payload = event.payload;
        if (payload.jobId !== jobId) return;
        
        if (payload.state === "Completed") {
           const intervals: SilenceInterval[] = JSON.parse(payload.message || "[]");
           const totalDurationMs = intervals.reduce((acc, curr) => acc + curr.durationMs, 0);
           setTestResult({ count: intervals.length, totalDurationMs });
           unlisten();
           setIsTesting(false);
        } else if (payload.state === "Failed" || payload.state === "Cancelled") {
           console.error("Silence detection failed/cancelled:", payload.error || payload.message);
           alert("Silence detection failed: " + (payload.error || payload.message));
           unlisten();
           setIsTesting(false);
        }
      });

      await invoke("start_silence_detection", {
        jobId,
        path: testVideoPath,
        settings: localConfig.settings
      });
    } catch (e) {
      console.error("IPC Error:", e);
      setIsTesting(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !isTesting && onOpenChange(open)}>
      <DialogContent className="sm:max-w-[550px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <VolumeX className="h-5 w-5 text-primary" />
            Cấu hình nhận diện khoảng lặng
          </DialogTitle>
          <DialogDescription>
            Điều chỉnh độ nhạy khi phát hiện khoảng lặng (Silence). Các thiết lập này xác định đoạn nào sẽ được đề xuất cắt bỏ.
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-6 py-4">
          <RadioGroup
            value={localConfig.preset}
            onValueChange={(val: string) => handlePresetChange(val as SilencePreset)}
            className="grid gap-3"
          >
            <div className="flex items-start space-x-3 rounded-lg border p-4 hover:bg-muted/50 transition-colors">
              <RadioGroupItem value="conservative" id="preset-conservative" className="mt-1" />
              <div className="grid gap-1.5">
                <Label htmlFor="preset-conservative" className="font-semibold cursor-pointer">
                  Cẩn trọng (Conservative)
                </Label>
                <p className="text-sm text-muted-foreground">
                  An toàn nhất, chỉ cắt những khoảng lặng rất rõ ràng và dài. Rất khó cắt lẹm vào chữ.
                </p>
              </div>
            </div>

            <div className="flex items-start space-x-3 rounded-lg border p-4 hover:bg-muted/50 transition-colors">
              <RadioGroupItem value="balanced" id="preset-balanced" className="mt-1" />
              <div className="grid gap-1.5">
                <Label htmlFor="preset-balanced" className="font-semibold cursor-pointer">
                  Cân bằng (Balanced) - Khuyên dùng
                </Label>
                <p className="text-sm text-muted-foreground">
                  Lý tưởng cho đa số video Vlog/Podcast, nhắm tới việc cắt bớt thời gian nghỉ thở tiêu chuẩn.
                </p>
              </div>
            </div>

            <div className="flex items-start space-x-3 rounded-lg border p-4 hover:bg-muted/50 transition-colors">
              <RadioGroupItem value="aggressive" id="preset-aggressive" className="mt-1" />
              <div className="grid gap-1.5">
                <Label htmlFor="preset-aggressive" className="font-semibold cursor-pointer">
                  Tích cực (Aggressive)
                </Label>
                <p className="text-sm text-muted-foreground">
                  Cắt sát từng nhịp nghỉ ngắn. Phù hợp cho video nhịp độ nhanh (Tiktok/Shorts) nhưng có thể cắt lẹm hơi thở.
                </p>
              </div>
            </div>

            <div className="flex items-start space-x-3 rounded-lg border p-4 hover:bg-muted/50 transition-colors">
              <RadioGroupItem value="custom" id="preset-custom" className="mt-1" />
              <div className="grid gap-1.5 w-full">
                <Label htmlFor="preset-custom" className="font-semibold cursor-pointer">
                  Tùy chỉnh (Advanced)
                </Label>
                <p className="text-sm text-muted-foreground mb-2">
                  Tự thiết lập ngưỡng cường độ âm thanh và thời gian.
                </p>
                
                {localConfig.preset === "custom" && (
                  <div className="space-y-6 pt-3 border-t mt-2">
                    <div className="grid gap-3">
                      <div className="flex justify-between items-center">
                        <Label>Ngưỡng âm lượng (Threshold)</Label>
                        <span className="text-xs text-muted-foreground font-mono bg-muted px-2 py-0.5 rounded">{localConfig.settings.thresholdDb} dB</span>
                      </div>
                      <input
                        type="range"
                        min="-60"
                        max="-10"
                        step="1"
                        value={localConfig.settings.thresholdDb}
                        onChange={(e) => setLocalConfig({
                          ...localConfig,
                          settings: { ...localConfig.settings, thresholdDb: parseInt(e.target.value) }
                        })}
                        className="w-full accent-primary"
                      />
                    </div>
                    
                    <div className="grid gap-3">
                      <div className="flex justify-between items-center">
                        <Label>Thời gian tối thiểu (Duration)</Label>
                        <span className="text-xs text-muted-foreground font-mono bg-muted px-2 py-0.5 rounded">{localConfig.settings.minDurationMs} ms</span>
                      </div>
                      <input
                        type="range"
                        min="100"
                        max="3000"
                        step="50"
                        value={localConfig.settings.minDurationMs}
                        onChange={(e) => setLocalConfig({
                          ...localConfig,
                          settings: { ...localConfig.settings, minDurationMs: parseInt(e.target.value) }
                        })}
                        className="w-full accent-primary"
                      />
                    </div>
                  </div>
                )}
              </div>
            </div>
          </RadioGroup>
        </div>

        <div className="flex items-center justify-between mt-2 pt-4 border-t">
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={handleTest}
              disabled={isTesting || !testVideoPath}
              className="gap-2"
            >
              {isTesting ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <PlayCircle className="h-4 w-4" />
              )}
              {isTesting ? "Đang chạy..." : "Chạy thử (Preview)"}
            </Button>

            {testResult && (
              <div className="text-sm text-muted-foreground animate-in fade-in slide-in-from-left-2 flex items-center gap-1.5 bg-primary/10 text-primary px-3 py-1.5 rounded-md">
                <CheckCircle2 className="h-4 w-4" />
                Dự kiến cắt: <span className="font-semibold">{testResult.count}</span> đoạn ({(testResult.totalDurationMs / 1000).toFixed(1)}s)
              </div>
            )}
          </div>
          
          <div className="flex gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={isTesting}>
              Hủy
            </Button>
            <Button onClick={handleSave} disabled={isTesting}>
              Lưu cấu hình
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
