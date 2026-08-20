import { useRef } from "react";
import { useProjectStore } from "../stores/useProjectStore";
import { MediaImporter } from "../components/media/MediaImporter";
import { VideoPreview, type VideoPreviewRef } from "../components/media/VideoPreview";
import { ExportPanel } from "../components/media/ExportPanel";
import { MediaRelink } from "../components/media/MediaRelink";
import { ReviewControls } from "../components/editor/ReviewControls";
import { DEFAULT_SILENCE_CONFIG } from "@/types/silence";

export function Home() {
  const { activeProject, updateProject, missingMediaIds } = useProjectStore();
  const videoPreviewRef = useRef<VideoPreviewRef>(null);

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

  const handlePreviewSuggestion = (startMs: number, endMs: number) => {
    videoPreviewRef.current?.playRange(startMs, endMs);
  };

  const handleEditPlanChange = (editPlan: typeof activeProject.editPlan) => {
    updateProject((draft) => {
      draft.editPlan = editPlan;
      draft.updatedAt = Date.now();
    });
  };

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

      {sourceData && isMissing && (
        <MediaRelink
          mediaId={sourceData.id}
          oldPath={sourceData.path}
          oldMetadata={sourceData.metadata}
        />
      )}

      {sourceData && !isMissing && (
        <div className="mt-4 flex flex-col items-start gap-4 xl:flex-row">
          <div className="max-w-xl flex-1">
            <VideoPreview
              ref={videoPreviewRef}
              path={sourceData.path}
              sourceMediaId={sourceData.id}
              editPlan={activeProject.editPlan}
              captionCues={activeProject.captionCues}
              captionStyle={activeProject.captions}
            />
            <ExportPanel />
          </div>

          <div className="h-[600px] w-full max-w-md flex-1">
            <ReviewControls
              mediaId={sourceData.id}
              sourcePath={sourceData.path}
              durationMs={Math.max(0, Math.round(sourceData.metadata.durationSec * 1000))}
              silenceConfig={activeProject.silenceSettings ?? DEFAULT_SILENCE_CONFIG}
              editPlan={activeProject.editPlan}
              onEditPlanChange={handleEditPlanChange}
              onPreview={handlePreviewSuggestion}
            />
          </div>
        </div>
      )}
    </div>
  );
}
