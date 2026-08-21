import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { VolumeX, Loader2, CheckCircle2, PlayCircle } from "lucide-react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "../ui/dialog";
import { Button } from "../ui/button";
import { RadioGroup, RadioGroupItem } from "../ui/radio-group";
import { Label } from "../ui/label";
import {
  SilencePreset,
  SilenceConfig,
  SilenceSettings,
  SilenceDetectionResult,
} from "@/types/silence";
import type { MediaJobEvent } from "@/services/media";
import { useI18n } from "@/i18n";

interface SilenceSettingsPanelProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  config: SilenceConfig;
  onChange: (config: SilenceConfig) => void;
  testVideoPath?: string; // Path to run detection against
}

const PRESET_MAPPING: Record<Exclude<SilencePreset, "custom">, SilenceSettings> = {
  conservative: { thresholdDb: -40, minDurationMs: 1500, paddingMs: 0 },
  balanced: { thresholdDb: -35, minDurationMs: 750, paddingMs: 0 },
  aggressive: { thresholdDb: -30, minDurationMs: 400, paddingMs: 0 },
};

export function SilenceSettingsPanel({
  isOpen,
  onOpenChange,
  config,
  onChange,
  testVideoPath,
}: SilenceSettingsPanelProps) {
  const { t } = useI18n();
  const [localConfig, setLocalConfig] = useState<SilenceConfig>(config);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ count: number; totalDurationMs: number } | null>(
    null,
  );

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
      alert(t("settings.noActiveVideo"));
      return;
    }
    try {
      setIsTesting(true);
      setTestResult(null);
      const jobId = `silence-test-${Date.now()}`;

      const unlisten = await listen<MediaJobEvent>("media-job", (event) => {
        const payload = event.payload;
        if (payload.jobId !== jobId) return;

        if (payload.state === "completed") {
          const result = payload.result as SilenceDetectionResult | undefined;
          const intervals = result?.intervals ?? [];
          const totalDurationMs = intervals.reduce((acc, curr) => acc + curr.durationMs, 0);
          setTestResult({ count: intervals.length, totalDurationMs });
          unlisten();
          setIsTesting(false);
        } else if (payload.state === "failed" || payload.state === "cancelled") {
          console.error("Silence detection failed/cancelled:", payload.error || payload.message);
          alert(`${t("settings.silenceFailed")} ${payload.error || payload.message}`);
          unlisten();
          setIsTesting(false);
        }
      });

      await invoke("start_silence_detection", {
        jobId,
        path: testVideoPath,
        settings: localConfig.settings,
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
            {t("settings.silenceTitle")}
          </DialogTitle>
          <DialogDescription>{t("settings.silenceDescription")}</DialogDescription>
        </DialogHeader>

        <div className="grid gap-6 py-4">
          <RadioGroup
            value={localConfig.preset}
            onValueChange={(val: string) => handlePresetChange(val as SilencePreset)}
            className="grid gap-3"
          >
            <div className="flex items-start space-x-3 rounded-lg border p-4 transition-colors hover:bg-muted/50">
              <RadioGroupItem value="conservative" id="preset-conservative" className="mt-1" />
              <div className="grid gap-1.5">
                <Label htmlFor="preset-conservative" className="cursor-pointer font-semibold">
                  {t("settings.conservative")}
                </Label>
                <p className="text-sm text-muted-foreground">
                  {t("settings.conservativeDescription")}
                </p>
              </div>
            </div>

            <div className="flex items-start space-x-3 rounded-lg border p-4 transition-colors hover:bg-muted/50">
              <RadioGroupItem value="balanced" id="preset-balanced" className="mt-1" />
              <div className="grid gap-1.5">
                <Label htmlFor="preset-balanced" className="cursor-pointer font-semibold">
                  {t("settings.balanced")}
                </Label>
                <p className="text-sm text-muted-foreground">{t("settings.balancedDescription")}</p>
              </div>
            </div>

            <div className="flex items-start space-x-3 rounded-lg border p-4 transition-colors hover:bg-muted/50">
              <RadioGroupItem value="aggressive" id="preset-aggressive" className="mt-1" />
              <div className="grid gap-1.5">
                <Label htmlFor="preset-aggressive" className="cursor-pointer font-semibold">
                  {t("settings.aggressive")}
                </Label>
                <p className="text-sm text-muted-foreground">
                  {t("settings.aggressiveDescription")}
                </p>
              </div>
            </div>

            <div className="flex items-start space-x-3 rounded-lg border p-4 transition-colors hover:bg-muted/50">
              <RadioGroupItem value="custom" id="preset-custom" className="mt-1" />
              <div className="grid w-full gap-1.5">
                <Label htmlFor="preset-custom" className="cursor-pointer font-semibold">
                  {t("settings.custom")}
                </Label>
                <p className="mb-2 text-sm text-muted-foreground">
                  {t("settings.customDescription")}
                </p>

                {localConfig.preset === "custom" && (
                  <div className="mt-2 space-y-6 border-t pt-3">
                    <div className="grid gap-3">
                      <div className="flex items-center justify-between">
                        <Label>{t("settings.volumeThreshold")}</Label>
                        <span className="rounded bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground">
                          {localConfig.settings.thresholdDb} dB
                        </span>
                      </div>
                      <input
                        type="range"
                        min="-60"
                        max="-10"
                        step="1"
                        value={localConfig.settings.thresholdDb}
                        onChange={(e) =>
                          setLocalConfig({
                            ...localConfig,
                            settings: {
                              ...localConfig.settings,
                              thresholdDb: parseInt(e.target.value),
                            },
                          })
                        }
                        className="w-full accent-primary"
                      />
                    </div>

                    <div className="grid gap-3">
                      <div className="flex items-center justify-between">
                        <Label>{t("settings.minimumDuration")}</Label>
                        <span className="rounded bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground">
                          {localConfig.settings.minDurationMs} ms
                        </span>
                      </div>
                      <input
                        type="range"
                        min="100"
                        max="3000"
                        step="50"
                        value={localConfig.settings.minDurationMs}
                        onChange={(e) =>
                          setLocalConfig({
                            ...localConfig,
                            settings: {
                              ...localConfig.settings,
                              minDurationMs: parseInt(e.target.value),
                            },
                          })
                        }
                        className="w-full accent-primary"
                      />
                    </div>
                  </div>
                )}
              </div>
            </div>
          </RadioGroup>
        </div>

        <div className="mt-2 flex items-center justify-between border-t pt-4">
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
              {isTesting ? t("settings.runningTest") : t("settings.runPreview")}
            </Button>

            {testResult && (
              <div className="flex items-center gap-1.5 rounded-md bg-primary/10 px-3 py-1.5 text-sm text-muted-foreground text-primary animate-in fade-in slide-in-from-left-2">
                <CheckCircle2 className="h-4 w-4" />
                {t("settings.expectedCuts", {
                  count: testResult.count,
                  duration: (testResult.totalDurationMs / 1000).toFixed(1),
                })}
              </div>
            )}
          </div>

          <div className="flex gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={isTesting}>
              {t("common.cancel")}
            </Button>
            <Button onClick={handleSave} disabled={isTesting}>
              {t("settings.saveConfiguration")}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
