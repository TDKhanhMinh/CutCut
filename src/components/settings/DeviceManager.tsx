import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/i18n";
import { deviceService, type DeviceRecord } from "@/services/device";
import { useAuthStore } from "@/stores/useAuthStore";

export function DeviceManager() {
  const { t } = useI18n();
  const user = useAuthStore((state) => state.user);
  const [devices, setDevices] = useState<DeviceRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!user) return;
    setLoading(true);
    setError(null);
    try {
      setDevices(await deviceService.list());
    } catch {
      setError(t("settings.devicesUnavailable"));
    } finally {
      setLoading(false);
    }
  }, [t, user]);

  useEffect(() => {
    const timer = window.setTimeout(() => void refresh(), 0);
    return () => window.clearTimeout(timer);
  }, [refresh]);

  const handleDeactivate = async (device: DeviceRecord) => {
    if (!window.confirm(t("settings.confirmDeactivateDevice"))) return;
    setLoading(true);
    setError(null);
    try {
      await deviceService.deactivate(device.id);
      await refresh();
    } catch {
      setError(t("settings.devicesUnavailable"));
      setLoading(false);
    }
  };

  if (!user) return null;

  return (
    <section className="mt-6 max-w-xl rounded-lg border border-border bg-card p-5">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h2 className="mb-2 text-lg font-semibold">{t("settings.devicesTitle")}</h2>
          <p className="text-sm text-muted-foreground">{t("settings.devicesDescription")}</p>
        </div>
        <Button variant="outline" size="sm" onClick={() => void refresh()} disabled={loading}>
          {loading ? t("common.loading") : t("common.refresh")}
        </Button>
      </div>

      {devices.length === 0 && !loading && (
        <p className="mt-4 text-sm text-muted-foreground">{t("settings.noDevices")}</p>
      )}
      <div className="mt-4 space-y-2">
        {devices.map((device) => (
          <div
            key={device.id}
            className="flex items-center justify-between gap-3 rounded-md border p-3"
          >
            <div className="min-w-0 text-sm">
              <p className="font-medium">{device.deviceLabel}</p>
              <p className="truncate text-muted-foreground">
                {device.platform} · {device.appVersion}
              </p>
            </div>
            {!device.isRevoked && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => void handleDeactivate(device)}
                disabled={loading}
              >
                {t("settings.deactivateDevice")}
              </Button>
            )}
          </div>
        ))}
      </div>
      {error && (
        <p className="mt-3 text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
