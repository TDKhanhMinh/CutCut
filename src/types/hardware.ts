export interface RuntimeProfile {
  cpuName: string;
  cpuLogicalCores: number;
  totalMemoryMb: number;
  hasAvx2: boolean;
  hasAvx512: boolean;
  hasGpu: boolean;
  gpuNames: string[];
  supportedAcceleration: string; // "CPU_AVX2" or "CPU_BASIC" for the V1 CPU bundle
  runtimeAvailable: boolean;
  runtimeVersion: string | null;
  runtimeBackends: string[];
  recommendedModelIds: string[];
  fallbackReason: string | null;
}

export type PresetType = "fast" | "balanced" | "accurate" | "custom";

export interface RuntimePresetPreference {
  schemaVersion: number;
  preset: PresetType;
  userOverrideModel: string | null;
}

export interface PresetResolution {
  preset: PresetType;
  targetModelId: string;
  targetBackend: string;
  isModelInstalled: boolean;
  fallbackReason: string | null;
  tradeoffDescription: string;
}
