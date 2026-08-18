import { useProjectStore } from "../stores/useProjectStore";
import { MediaImporter } from "../components/media/MediaImporter";
import { VideoPreview } from "../components/media/VideoPreview";
import { ExportPanel } from "../components/media/ExportPanel";
import { MediaRelink } from "../components/media/MediaRelink";

export function Home() {
  const { activeProject, updateProject, missingMediaIds } = useProjectStore();

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

  return (
    <div className="flex-1 p-8 overflow-y-auto">
      <h1 className="mb-4 text-2xl font-bold">CutCut Media Toolchain</h1>
      <p className="text-muted-foreground mb-8">
        Prototype proving native file dialog, FFprobe metadata parsing, local video preview (Asset Protocol), and FFmpeg export jobs.
      </p>
      
      {/* 
        Temporarily adapting MediaImporter to update the project media array
        instead of local component state, so it triggers Autosave.
      */}
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
            <VideoPreview path={sourceData.path} />
            <ExportPanel inputPath={sourceData.path} totalDurationSec={sourceData.metadata.durationSec} />
        </div>
      )}
    </div>
  );
}
