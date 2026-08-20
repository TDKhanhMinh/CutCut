export type ResourceState =
  | "notInstalled"
  | "installed"
  | {
      downloading: { progress: number; downloaded: number; total: number };
    }
  | { incompatible: { reason: string } }
  | { corrupted: { reason: string } }
  | { failed: { reason: string } };

export interface ResourceCompatibility {
  minMemoryMb: number;
  requiresAvx2: boolean;
  supportedBackends: string[];
  runtimeVersion: string;
}

export interface ResourceItem {
  id: string;
  resourceType: "whisper-model" | "vad-model" | "runtime-asset" | "other";
  name: string;
  version: string;
  sizeBytes: number;
  url: string;
  checksum: string;
  compatibility: ResourceCompatibility;
}
