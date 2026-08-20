export type ArtifactType =
  | "mediaMetadata"
  | "extractedAudio"
  | "transcript"
  | "silenceAnalysis"
  | "localAnalysis"
  | "aiAnalysis"
  | "caption"
  | "preview";

export const ARTIFACT_SIGNATURE_ALGORITHM = "sha256-canonical-json-v1" as const;

/**
 * Inputs use the Project-wide millisecond timestamp convention (`*Ms`).
 * Rust owns hashing; this type keeps the frontend descriptor in sync.
 */
export interface ArtifactSignatureInput {
  artifactType: ArtifactType;
  artifactVersion: number;
  dependsOn: string[];
  inputs: Record<string, unknown>;
}

export interface ArtifactSignature {
  artifactType: ArtifactType;
  artifactVersion: number;
  signature: string;
  dependsOn: string[];
  inputs: Record<string, unknown>;
}

export type ArtifactDescriptor = ArtifactSignature;

export type ArtifactStatus = "valid" | "stale" | "missing" | "building" | "failed";

export type ArtifactDiagnosticReason =
  "dependencyChanged" | "fileMissing" | "integrityMismatch" | "invalidPath" | "registrationFailed";

export interface ArtifactRecord {
  id: string;
  artifactType: ArtifactType;
  signature: string;
  relativePath: string;
  createdAt: number;
  artifactVersion: number;
  producer: string;
  status: ArtifactStatus;
  dependencies: string[];
  integrity?: string | null;
  diagnosticReason?: ArtifactDiagnosticReason | null;
}
