import { useState } from "react";
import { telemetry } from "@/services/telemetry";
import { useI18n } from "@/i18n";

export function TelemetrySettings() {
  const { t } = useI18n();
  const [enabled, setEnabled] = useState(() => telemetry.isEnabled());
  const toggle = (next: boolean) => {
    telemetry.setEnabled(next);
    setEnabled(next);
  };

  return (
    <section className="mt-6 max-w-xl rounded-lg border border-border bg-card p-5">
      <h2 className="mb-2 text-lg font-semibold">{t("settings.telemetryTitle")}</h2>
      <p className="mb-4 text-sm text-muted-foreground">{t("settings.telemetryDescription")}</p>
      <label className="flex items-center gap-3 text-sm">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(event) => toggle(event.target.checked)}
        />
        {t("settings.telemetryOptIn")}
      </label>
      <p className="mt-3 text-xs text-muted-foreground">{t("settings.telemetryDisabled")}</p>
    </section>
  );
}
