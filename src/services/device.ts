import { invoke } from "@tauri-apps/api/core";
import { authService } from "@/services/auth";

const DEVICE_LABEL = "CutCut Desktop";
const PLATFORM = "windows";
const APP_VERSION = import.meta.env.VITE_APP_VERSION || "0.1.0";

export interface DeviceRecord {
  id: string;
  deviceLabel: string;
  appVersion: string;
  platform: string;
  lastActiveAt: string;
  isRevoked: boolean;
  createdAt: string;
}

interface ActivationResponse {
  activated: boolean;
  deviceId: string;
  deviceLimit: number;
}

interface DeviceListResponse {
  devices: Array<{
    id: string;
    device_label: string;
    app_version: string;
    platform: string;
    last_active_at: string;
    is_revoked: boolean;
    created_at: string;
  }>;
}

let installationIdPromise: Promise<string> | null = null;
let currentDeviceId: string | null = null;

function asBody(value: object): Record<string, unknown> {
  return value as Record<string, unknown>;
}

async function getInstallationId(): Promise<string> {
  if (!installationIdPromise) {
    installationIdPromise = invoke<string>("get_or_create_installation_id");
  }
  return installationIdPromise;
}

async function hashInstallationId(value: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function invokeDevice<T>(body: object): Promise<T> {
  const { data, error } = await authService.invokeFunction<T>("device-activation", asBody(body));
  if (error || !data) throw new Error(error?.message ?? "device_unavailable");
  return data;
}

export const deviceService = {
  async activate(): Promise<ActivationResponse> {
    const installationId = await getInstallationId();
    const response = await invokeDevice<ActivationResponse>({
      action: "activate",
      deviceHash: await hashInstallationId(installationId),
      deviceLabel: DEVICE_LABEL,
      appVersion: APP_VERSION,
      platform: PLATFORM,
    });
    currentDeviceId = response.deviceId;
    return response;
  },

  async list(): Promise<DeviceRecord[]> {
    const response = await invokeDevice<DeviceListResponse>({ action: "status" });
    return response.devices.map((device) => ({
      id: device.id,
      deviceLabel: device.device_label,
      appVersion: device.app_version,
      platform: device.platform,
      lastActiveAt: device.last_active_at,
      isRevoked: device.is_revoked,
      createdAt: device.created_at,
    }));
  },

  async deactivate(deviceId = currentDeviceId): Promise<void> {
    if (!deviceId) throw new Error("device_not_selected");
    const response = await invokeDevice<{ deactivated: boolean }>({
      action: "deactivate",
      deviceId,
    });
    if (!response.deactivated) throw new Error("device_not_found");
    if (deviceId === currentDeviceId) currentDeviceId = null;
  },
};
