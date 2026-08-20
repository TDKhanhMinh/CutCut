import { useProjectStore } from "../stores/useProjectStore";
import { MediaImporter } from "../components/media/MediaImporter";
import { VideoPreview } from "../components/media/VideoPreview";
import { ExportPanel } from "../components/media/ExportPanel";

export function Home() {
  const { activeProject, updateProject } = useProjectStore();

  if (!activeProject) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <div className="text-center">
          <h1 className="mb-4 text-3xl font-bold">Welcome to CutCut</h1>
          <p className="mb-8 text-muted-foreground">
            Please create a new project or open an existing one from the sidebar.
          </p>
        </div>
      </div>
    );
  }

  // We use the first media source as our active demo for now
  const sourceData = activeProject.media.length > 0 ? activeProject.media[0] : null;

  return (
    <div className="flex-1 overflow-y-auto p-8">
      <h1 className="mb-4 text-2xl font-bold">CutCut Media Toolchain</h1>
      <p className="mb-8 text-muted-foreground">
        Prototype proving native file dialog, FFprobe metadata parsing, local video preview (Asset
        Protocol), and FFmpeg export jobs.
      </p>

      {/* 
        Temporarily adapting MediaImporter to update the project media array
        instead of local component state, so it triggers Autosave.
      */}
      <MediaImporter
        onMetadataParsed={(meta) => {
          updateProject((draft) => {
            draft.media = [
              {
                id: "demo-media-1",
                path: meta.path,
                metadata: meta,
              },
            ];
          });
        }}
      />

      {sourceData && (
        <div className="flex flex-col items-start gap-4 xl:flex-row">
          <VideoPreview path={sourceData.path} />
          <ExportPanel
            inputPath={sourceData.path}
            totalDurationSec={sourceData.metadata.durationSec}
          />
        </div>
      )}
    </div>
  );
}
