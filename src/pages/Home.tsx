import { useState } from "react";
import { MediaImporter, MediaSourceMetadata } from "../components/media/MediaImporter";
import { VideoPreview } from "../components/media/VideoPreview";
import { ExportPanel } from "../components/media/ExportPanel";

export function Home() {
  const [sourceData, setSourceData] = useState<MediaSourceMetadata | null>(null);

  return (
    <div className="flex-1 p-8 overflow-y-auto">
      <h1 className="mb-4 text-2xl font-bold">CutCut Media Toolchain</h1>
      <p className="text-muted-foreground mb-8">
        Prototype proving native file dialog, FFprobe metadata parsing, local video preview (Asset Protocol), and FFmpeg export jobs.
      </p>
      
      <MediaImporter onMetadataParsed={(meta) => setSourceData(meta)} />

      {sourceData && (
        <div className="flex flex-col xl:flex-row gap-4 items-start">
            <VideoPreview path={sourceData.path} />
            <ExportPanel inputPath={sourceData.path} totalDurationSec={sourceData.durationSec} />
        </div>
      )}
    </div>
  );
}
