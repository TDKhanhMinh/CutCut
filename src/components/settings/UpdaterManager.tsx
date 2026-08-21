import { useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useI18n } from "@/i18n";

export function UpdaterManager() {
  const { t } = useI18n();
  const [update, setUpdate] = useState<Update | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const checkForUpdate = async () => {
    setLoading(true);
    setStatus(null);
    try {
      const available = await check();
      setUpdate(available);
      setStatus(
        available
          ? t("settings.updateAvailable", { version: available.version })
          : t("settings.upToDate"),
      );
    } catch {
      // A missing endpoint/signing key is intentionally fail-closed in dev.
      setStatus(t("settings.updaterUnconfigured"));
    } finally {
      setLoading(false);
    }
  };

  const installUpdate = async () => {
    if (!update) return;
    setLoading(true);
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch {
      setStatus(t("settings.updateError"));
      setLoading(false);
    }
  };

  return (
    <section className="mt-6 max-w-xl rounded-lg border border-border bg-card p-5">
      <h2 className="mb-2 text-lg font-semibold">{t("settings.updaterTitle")}</h2>
      <p className="mb-4 text-sm text-muted-foreground">{t("settings.updaterUnconfigured")}</p>
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className="rounded-md border border-border px-3 py-2 text-sm disabled:opacity-50"
          onClick={() => void checkForUpdate()}
          disabled={loading}
        >
          {loading ? t("settings.checkingUpdates") : t("settings.checkUpdates")}
        </button>
        {update && (
          <button
            type="button"
            className="rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground disabled:opacity-50"
            onClick={() => void installUpdate()}
            disabled={loading}
          >
            {t("settings.updateAvailable", { version: update.version })}
          </button>
        )}
      </div>
      {status && <p className="mt-3 text-sm text-muted-foreground">{status}</p>}
    </section>
  );
}
