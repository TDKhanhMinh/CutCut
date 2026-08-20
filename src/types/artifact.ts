export type ArtifactType =
  "transcript" | "silenceAnalysis" | "preview" | "caption" | "extractedAudio";

export interface ArtifactSignature {
  artifactType: ArtifactType;
  artifactVersion: number;
  signature: string;
  dependsOn: string[];
  inputs: Record<string, unknown>;
}

export type ArtifactStatus = "valid" | "stale" | "missing" | "building" | "failed";

export interface ArtifactRecord {
  id: string;
  artifactType: ArtifactType;
  signature: string;
  relativePath: string;
  createdAt: number;
  status: ArtifactStatus;
  dependencies: string[];
}
