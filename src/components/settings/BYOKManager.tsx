import { useEffect, useState } from "react";
import { useAIConfigStore } from "@/stores/useAIConfigStore";
import { translateErrorCode, useI18n } from "@/i18n";

export function BYOKManager() {
  const { locale, t } = useI18n();
  const { mode, keyStatus, loading, error, hydrate, setMode, saveKey, removeKey, testKey } =
    useAIConfigStore();
  const [apiKey, setApiKey] = useState("");
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  const handleSave = async () => {
    setMessage(null);
    try {
      await saveKey(apiKey);
      setApiKey("");
      setMessage(t("settings.keySaved"));
    } catch {
      // Store exposes the redacted error to the UI.
    }
  };

  const handleTest = async () => {
    setMessage(null);
    try {
      await testKey();
      setMessage(t("settings.connectionSuccess"));
    } catch {
      // Store exposes the redacted error to the UI.
    }
  };

  return (
    <section className="mt-6 max-w-xl rounded-lg border border-border bg-card p-5">
      <h2 className="mb-2 text-lg font-semibold">{t("settings.aiTitle")}</h2>
      <p className="mb-4 text-sm text-muted-foreground">{t("settings.byokDescription")}</p>

      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className={`rounded-md border px-3 py-2 text-sm ${mode === "hosted" ? "border-primary bg-primary/10" : ""}`}
          onClick={() => setMode("hosted")}
        >
          {t("settings.hostedMode")}
        </button>
        <button
          type="button"
          className={`rounded-md border px-3 py-2 text-sm ${mode === "byok" ? "border-primary bg-primary/10" : ""}`}
          onClick={() => setMode("byok")}
        >
          {t("settings.byokMode")}
        </button>
      </div>

      <div className="mt-4 space-y-3">
        <p className="text-sm">
          {keyStatus.configured
            ? `${t("settings.keyConfigured")} ${keyStatus.maskedHint ?? ""}`
            : t("settings.keyNotConfigured")}
        </p>
        <label className="block text-sm">
          <span className="mb-1 block text-muted-foreground">{t("settings.keyLabel")}</span>
          <input
            type="password"
            value={apiKey}
            onChange={(event) => setApiKey(event.target.value)}
            autoComplete="new-password"
            placeholder={t("settings.keyPlaceholder")}
            className="w-full rounded-md border border-border bg-background px-3 py-2"
          />
        </label>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground disabled:opacity-50"
            onClick={() => void handleSave()}
            disabled={loading || apiKey.trim().length === 0}
          >
            {t("settings.saveKey")}
          </button>
          <button
            type="button"
            className="rounded-md border border-border px-3 py-2 text-sm disabled:opacity-50"
            onClick={() => void handleTest()}
            disabled={loading || !keyStatus.configured}
          >
            {t("common.testConnection")}
          </button>
          <button
            type="button"
            className="rounded-md border border-destructive px-3 py-2 text-sm text-destructive disabled:opacity-50"
            onClick={() => void removeKey()}
            disabled={loading || !keyStatus.configured}
          >
            {t("settings.removeKey")}
          </button>
        </div>
      </div>

      {message && <p className="mt-3 text-sm text-green-600">{message}</p>}
      {error && (
        <p className="mt-3 text-sm text-destructive" role="alert">
          {translateErrorCode(error, locale)}
        </p>
      )}
    </section>
  );
}
