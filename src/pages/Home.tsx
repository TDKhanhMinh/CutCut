import { useRef } from "react";
import { useProjectStore } from "../stores/useProjectStore";
import { MediaImporter } from "../components/media/MediaImporter";
import { VideoPreview, VideoPreviewRef } from "../components/media/VideoPreview";
import { ExportPanel } from "../components/media/ExportPanel";
import { MediaRelink } from "../components/media/MediaRelink";
import { ReviewControls } from "../components/editor/ReviewControls";

export function Home() {
  const { activeProject, updateProject, missingMediaIds } = useProjectStore();
  const videoPreviewRef = useRef<VideoPreviewRef>(null);

  if (!activeProject) {
    return (
      <div className="flex-1 p-8 flex items-center justify-center">
        <div className="text-center">
          <h1 className="text-3xl font-bold mb-4">Welcome to CutCut</h1>
          <p className="text-muted-foreground mb-8">Please create a new project or open an existing one from the sidebar.</p>
        </div>
      </div>
    );
  }

  // We use the first media source as our active demo for now
  const sourceData = activeProject.media.length > 0 ? activeProject.media[0] : null;
  const isMissing = sourceData ? missingMediaIds.includes(sourceData.id) : false;

  const handlePreviewSuggestion = (startMs: number, endMs: number) => {
      videoPreviewRef.current?.playRange(startMs, endMs);
  };

  return (
    <div className="flex-1 p-8 overflow-y-auto">
      <h1 className="mb-4 text-2xl font-bold">CutCut Media Toolchain</h1>
      <p className="text-muted-foreground mb-8">
        Prototype proving native file dialog, FFprobe metadata parsing, local video preview (Asset Protocol), and FFmpeg export jobs.
      </p>
      
      {!sourceData && (
        <MediaImporter onMetadataParsed={(meta) => {
            updateProject((draft) => {
                draft.media = [{
                    id: "demo-media-1",
                    path: meta.path,
                    metadata: meta
                }];
            });
        }} />
      )}

      {sourceData && isMissing && (
        <MediaRelink mediaId={sourceData.id} oldPath={sourceData.path} />
      )}

      {sourceData && !isMissing && (
        <div className="flex flex-col xl:flex-row gap-4 items-start mt-4">
            <div className="flex-1 max-w-xl">
                <VideoPreview ref={videoPreviewRef} path={sourceData.path} editPlan={activeProject.editPlan} />
                <ExportPanel inputPath={sourceData.path} totalDurationSec={sourceData.metadata.durationSec} />
            </div>
            
            <div className="flex-1 w-full max-w-md h-[600px]">
                <ReviewControls mediaId={sourceData.id} onPreview={handlePreviewSuggestion} />
            </div>
        </div>
      )}
    </div>
  );
}
