import { useEffect, useState } from "react";
import { clearProjectCache, getCacheUsage } from "@/services/cache";
import {
  getRuntimePresetPreference,
  getRuntimeProfile,
  resolveRuntimePreset,
  setRuntimePresetPreference,
} from "@/services/runtime";
import { useProjectStore } from "@/stores/useProjectStore";
import { BYOKManager } from "@/components/settings/BYOKManager";
import { LocaleSelector } from "@/components/settings/LocaleSelector";
import { formatBytes, useI18n } from "@/i18n";
import { UpdaterManager } from "@/components/settings/UpdaterManager";
import { TelemetrySettings } from "@/components/settings/TelemetrySettings";
import type {
  PresetResolution,
  PresetType,
  RuntimePresetPreference,
  RuntimeProfile,
} from "@/types/hardware";

export function Settings() {
  const { locale, t } = useI18n();
  const activeProject = useProjectStore((state) => state.activeProject);
  const projectPath = useProjectStore((state) => state.projectPath);
  const setProject = useProjectStore((state) => state.setProject);
  const [reclaimableBytes, setReclaimableBytes] = useState(0);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [runtimeProfile, setRuntimeProfile] = useState<RuntimeProfile | null>(null);
  const [runtimeLoading, setRuntimeLoading] = useState(false);
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [presetPreference, setPresetPreference] = useState<RuntimePresetPreference | null>(null);
  const [presetResolution, setPresetResolution] = useState<PresetResolution | null>(null);
  const [presetOverride, setPresetOverride] = useState("");
  const [presetLoading, setPresetLoading] = useState(false);
  const [presetError, setPresetError] = useState<string | null>(null);

  useEffect(() => {
    if (!activeProject || !projectPath) return;

    let cancelled = false;
    void getCacheUsage(projectPath, activeProject)
      .then((usage) => {
        if (!cancelled) setReclaimableBytes(usage.reclaimableBytes);
      })
      .catch((cause: unknown) => {
        if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
      });

    return () => {
      cancelled = true;
    };
  }, [activeProject, projectPath]);

  useEffect(() => {
    let cancelled = false;
    void getRuntimeProfile()
      .then((profile) => {
        if (!cancelled) setRuntimeProfile(profile);
      })
      .catch((cause: unknown) => {
        if (!cancelled) {
          setRuntimeError(cause instanceof Error ? cause.message : String(cause));
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    void getRuntimePresetPreference()
      .then(async (preference) => {
        const resolution = await resolveRuntimePreset(
          preference.preset,
          preference.userOverrideModel,
        );
        if (!cancelled) {
          setPresetPreference(preference);
          setPresetOverride(preference.userOverrideModel ?? "");
          setPresetResolution(resolution);
        }
      })
      .catch((cause: unknown) => {
        if (!cancelled) {
          setPresetError(cause instanceof Error ? cause.message : String(cause));
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const handleClearCache = async () => {
    if (!activeProject || !projectPath) return;

    setLoading(true);
    setError(null);
    setMessage(null);
    try {
      const result = await clearProjectCache(projectPath, activeProject);
      setProject(result.project, projectPath);
      setReclaimableBytes(0);
      setMessage(t("settings.cacheCleared", { bytes: formatBytes(result.freedBytes, locale) }));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  };

  const handleRefreshRuntime = async () => {
    setRuntimeLoading(true);
    setRuntimeError(null);
    try {
      setRuntimeProfile(await getRuntimeProfile(true));
    } catch (cause) {
      setRuntimeError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setRuntimeLoading(false);
    }
  };

  const handlePresetChange = async (preset: PresetType) => {
    setPresetLoading(true);
    setPresetError(null);
    try {
      const preference = await setRuntimePresetPreference(preset, presetOverride.trim() || null);
      setPresetPreference(preference);
      setPresetResolution(
        await resolveRuntimePreset(preference.preset, preference.userOverrideModel),
      );
    } catch (cause) {
      setPresetError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setPresetLoading(false);
    }
  };

  return (
    <div className="flex-1 p-8">
      <h1 className="mb-4 text-2xl font-bold">{t("common.settings")}</h1>

      <LocaleSelector />

      <section className="max-w-xl rounded-lg border border-border bg-card p-5">
        <h2 className="mb-2 text-lg font-semibold">{t("settings.cacheTitle")}</h2>
        <p className="mb-4 text-sm text-muted-foreground">{t("settings.cacheDescription")}</p>

        {!activeProject || !projectPath ? (
          <p className="text-sm text-muted-foreground">{t("settings.openProjectForCache")}</p>
        ) : (
          <div className="space-y-3">
            <p className="text-sm">
              {t("settings.reclaimable")} <strong>{formatBytes(reclaimableBytes, locale)}</strong>
            </p>
            <button
              type="button"
              className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
              onClick={handleClearCache}
              disabled={loading || reclaimableBytes === 0}
            >
              {loading ? t("settings.clearingCache") : t("settings.clearCache")}
            </button>
          </div>
        )}

        {message && <p className="mt-4 text-sm text-green-600">{message}</p>}
        {error && (
          <p className="mt-4 text-sm text-destructive" role="alert">
            {error}
          </p>
        )}
      </section>

      <section className="mt-6 max-w-xl rounded-lg border border-border bg-card p-5">
        <div className="flex items-center justify-between gap-4">
          <div>
            <h2 className="mb-2 text-lg font-semibold">{t("settings.runtimeTitle")}</h2>
            <p className="text-sm text-muted-foreground">{t("settings.runtimeDescription")}</p>
          </div>
          <button
            type="button"
            className="shrink-0 rounded-md border border-border px-3 py-2 text-sm hover:bg-muted disabled:opacity-50"
            onClick={handleRefreshRuntime}
            disabled={runtimeLoading}
          >
            {runtimeLoading ? t("settings.checkingRuntime") : t("settings.refreshRuntime")}
          </button>
        </div>

        {runtimeProfile ? (
          <dl className="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
            <dt className="text-muted-foreground">{t("settings.cpu")}</dt>
            <dd>{runtimeProfile.cpuName}</dd>
            <dt className="text-muted-foreground">RAM</dt>
            <dd>{formatBytes(runtimeProfile.totalMemoryMb * 1024 * 1024, locale)}</dd>
            <dt className="text-muted-foreground">{t("settings.whisperRuntime")}</dt>
            <dd>
              {runtimeProfile.runtimeAvailable
                ? (runtimeProfile.runtimeVersion ?? t("common.available"))
                : t("common.unavailable")}
            </dd>
            <dt className="text-muted-foreground">{t("settings.backend")}</dt>
            <dd>{runtimeProfile.supportedAcceleration}</dd>
            <dt className="text-muted-foreground">{t("settings.modelTiers")}</dt>
            <dd>{runtimeProfile.recommendedModelIds.join(", ")}</dd>
            <dt className="text-muted-foreground">{t("settings.gpu")}</dt>
            <dd>
              {runtimeProfile.gpuNames.length > 0
                ? runtimeProfile.gpuNames.join(", ")
                : t("settings.noGpu")}
            </dd>
          </dl>
        ) : (
          <p className="mt-4 text-sm text-muted-foreground">{t("settings.readingRuntime")}</p>
        )}

        {runtimeProfile?.fallbackReason && (
          <p className="mt-4 text-sm text-amber-600">
            {t("settings.fallback")} {runtimeProfile.fallbackReason}
          </p>
        )}
        {runtimeError && (
          <p className="mt-4 text-sm text-destructive" role="alert">
            {runtimeError}
          </p>
        )}
      </section>

      <section className="mt-6 max-w-xl rounded-lg border border-border bg-card p-5">
        <h2 className="mb-2 text-lg font-semibold">{t("settings.speechTitle")}</h2>
        <p className="mb-4 text-sm text-muted-foreground">{t("settings.speechDescription")}</p>
        <div className="flex flex-wrap gap-2">
          {(["fast", "balanced", "accurate"] as PresetType[]).map((preset) => (
            <button
              key={preset}
              type="button"
              className={`rounded-md border px-3 py-2 text-sm capitalize hover:bg-muted disabled:opacity-50 ${
                presetPreference?.preset === preset ? "border-primary bg-primary/10" : ""
              }`}
              onClick={() => void handlePresetChange(preset)}
              disabled={presetLoading}
            >
              {preset}
            </button>
          ))}
        </div>

        <label className="mt-4 block text-sm">
          <span className="mb-1 block text-muted-foreground">{t("settings.advancedOverride")}</span>
          <input
            value={presetOverride}
            onChange={(event) => setPresetOverride(event.target.value)}
            placeholder="ggml-tiny / ggml-base / ggml-small"
            className="w-full rounded-md border border-border bg-background px-3 py-2"
          />
        </label>
        <button
          type="button"
          className="mt-3 rounded-md border border-border px-3 py-2 text-sm hover:bg-muted disabled:opacity-50"
          onClick={() => void handlePresetChange(presetPreference?.preset ?? "balanced")}
          disabled={presetLoading || !presetPreference}
        >
          {presetLoading ? t("settings.savingPreset") : t("settings.savePreset")}
        </button>

        {presetResolution && (
          <div className="mt-4 rounded-md bg-muted/40 p-3 text-sm">
            <p>
              {t("settings.recommendation")} <strong>{presetResolution.targetModelId}</strong> ·
              backend <strong>{presetResolution.targetBackend}</strong>
            </p>
            <p className="mt-1 text-muted-foreground">{presetResolution.tradeoffDescription}</p>
            {!presetResolution.isModelInstalled && (
              <p className="mt-2 text-amber-600">{t("settings.modelNotReady")}</p>
            )}
            {presetResolution.fallbackReason && (
              <p className="mt-2 text-amber-600">
                {t("settings.fallback")} {presetResolution.fallbackReason}
              </p>
            )}
          </div>
        )}
        {presetError && (
          <p className="mt-3 text-sm text-destructive" role="alert">
            {presetError}
          </p>
        )}
      </section>

      <BYOKManager />
      <UpdaterManager />
      <TelemetrySettings />
    </div>
  );
}
