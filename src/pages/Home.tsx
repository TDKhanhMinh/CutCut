import { useProjectStore } from "../stores/useProjectStore";
import { MediaImporter } from "../components/media/MediaImporter";
import { VideoPreview } from "../components/media/VideoPreview";
import { ExportPanel } from "../components/media/ExportPanel";
import { MediaRelink } from "../components/media/MediaRelink";

export function Home() {
  const { activeProject, updateProject, missingMediaIds } = useProjectStore();

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

  const sourceData = activeProject.media.length > 0 ? activeProject.media[0] : null;
  const isMissing = sourceData ? missingMediaIds.includes(sourceData.id) : false;

  return (
    <div className="flex-1 overflow-y-auto p-8">
      <h1 className="mb-4 text-2xl font-bold">CutCut Media Toolchain</h1>
      <p className="mb-8 text-muted-foreground">
        Prototype proving native file dialog, FFprobe metadata parsing, local video preview (Asset
        Protocol), and FFmpeg export jobs.
      </p>

      {!sourceData && (
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
      )}

      {sourceData && isMissing && <MediaRelink mediaId={sourceData.id} oldPath={sourceData.path} />}

      {sourceData && !isMissing && (
        <div className="mt-4 flex flex-col items-start gap-4 xl:flex-row">
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
