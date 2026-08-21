export type TelemetryEventName =
  | "startup"
  | "stt_started"
  | "stt_completed"
  | "stt_failed"
  | "export_started"
  | "export_completed"
  | "export_failed"
  | "updater_check";

export interface TelemetryEvent {
  name: TelemetryEventName;
  timestamp: string;
  appVersion: string;
  properties: Record<string, boolean | number | string | null>;
}

const enabledKey = "cutcut-telemetry-enabled";
const queueKey = "cutcut-telemetry-queue";
const allowedProperties = new Set([
  "durationMs",
  "modelId",
  "backend",
  "status",
  "errorCode",
  "cancelled",
  "updateAvailable",
]);

const sensitiveValuePatterns = [
  /Bearer\s+[A-Za-z0-9._-]+/gi,
  /eyJ[A-Za-z0-9_-]{12,}\.[A-Za-z0-9._-]{12,}/g,
  /AIza[0-9A-Za-z_-]{20,}/g,
  /(?:[A-Za-z]:[\\/]|\\\\|\/Users\/|\/home\/)[^\s"']+/gi,
];

export function sanitizeTelemetryString(value: string): string {
  return sensitiveValuePatterns
    .reduce((result, pattern) => result.replace(pattern, "[redacted]"), value)
    .slice(0, 80);
}

function isEnabled(): boolean {
  if (typeof window === "undefined") return false;
  return window.localStorage.getItem(enabledKey) !== "false";
}

export function redactTelemetryProperties(
  properties: Record<string, unknown>,
): Record<string, boolean | number | string | null> {
  return Object.fromEntries(
    Object.entries(properties)
      .filter(([key]) => allowedProperties.has(key))
      .map(([key, value]) => {
        if (value === null || typeof value === "boolean" || typeof value === "number") {
          return [key, value];
        }
        return [key, typeof value === "string" ? sanitizeTelemetryString(value) : null];
      }),
  );
}

function readQueue(): TelemetryEvent[] {
  if (typeof window === "undefined") return [];
  try {
    const parsed = JSON.parse(window.localStorage.getItem(queueKey) ?? "[]");
    return Array.isArray(parsed) ? (parsed as TelemetryEvent[]).slice(-49) : [];
  } catch {
    return [];
  }
}

function writeQueue(events: TelemetryEvent[]) {
  if (typeof window !== "undefined")
    window.localStorage.setItem(queueKey, JSON.stringify(events.slice(-50)));
}

async function flushQueue(): Promise<void> {
  if (!isEnabled() || typeof window === "undefined") return;
  const endpoint = import.meta.env.VITE_TELEMETRY_ENDPOINT as string | undefined;
  if (!endpoint) return;
  const events = readQueue();
  if (events.length === 0) return;
  try {
    const result = await fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ events }),
      keepalive: true,
    });
    if (result.ok) writeQueue([]);
  } catch {
    // Telemetry is best-effort; network failure must never block the editor.
  }
}

export const telemetry = {
  isEnabled,
  setEnabled(enabled: boolean) {
    if (typeof window !== "undefined") window.localStorage.setItem(enabledKey, String(enabled));
    if (!enabled) writeQueue([]);
    else void flushQueue();
  },
  getQueuedEvents: readQueue,
  flush: flushQueue,
  track(name: TelemetryEventName, properties: Record<string, unknown> = {}) {
    if (!isEnabled()) return;
    const event: TelemetryEvent = {
      name,
      timestamp: new Date().toISOString(),
      appVersion: import.meta.env.VITE_APP_VERSION ?? "0.1.0",
      properties: redactTelemetryProperties(properties),
    };
    writeQueue([...readQueue(), event]);
    void flushQueue();
  },
};
