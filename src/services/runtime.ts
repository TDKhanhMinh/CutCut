import { invoke } from "@tauri-apps/api/core";
import type {
  PresetResolution,
  PresetType,
  RuntimePresetPreference,
  RuntimeProfile,
} from "@/types/hardware";

export function getRuntimeProfile(refresh = false): Promise<RuntimeProfile> {
  return invoke<RuntimeProfile>("get_runtime_profile", { refresh });
}

export function resolveRuntimePreset(
  preset: PresetType,
  userOverrideModel: string | null,
): Promise<PresetResolution> {
  return invoke<PresetResolution>("resolve_runtime_preset", {
    preset,
    userOverride: userOverrideModel,
  });
}

export function getRuntimePresetPreference(): Promise<RuntimePresetPreference> {
  return invoke<RuntimePresetPreference>("get_runtime_preset_preference");
}

export function setRuntimePresetPreference(
  preset: PresetType,
  userOverrideModel: string | null,
): Promise<RuntimePresetPreference> {
  return invoke<RuntimePresetPreference>("set_runtime_preset_preference", {
    preset,
    userOverrideModel,
  });
}
