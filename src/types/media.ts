export interface MediaSourceMetadata {
  path: string;
  durationSec: number;
  fps: number;
  width: number;
  height: number;
  videoCodec: string;
  audioCodec?: string;
  rotation: number;
}
