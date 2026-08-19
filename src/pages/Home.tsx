import { useRef } from "react";
import { useProjectStore } from "../stores/useProjectStore";
import { MediaImporter } from "../components/media/MediaImporter";
import { VideoPreview, VideoPreviewRef } from "../components/media/VideoPreview";
import { ExportPanel } from "../components/media/ExportPanel";
import { MediaRelink } from "../components/media/MediaRelink";
import { EditReviewPanel } from "../components/editor/EditReviewPanel";

import { Undo2, Redo2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useStore } from "zustand";

export function Home() {
  const { activeProject, updateProject, missingMediaIds } = useProjectStore();
  const { undo, redo, pastStates, futureStates } = useStore(useProjectStore.temporal, state => state);
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
      <div className="flex justify-between items-start mb-4">
        <div>
          <h1 className="text-2xl font-bold mb-1">CutCut Media Toolchain</h1>
          <p className="text-muted-foreground mb-8">
            Prototype proving native file dialog, FFprobe metadata parsing, local video preview (Asset Protocol), and FFmpeg export jobs.
          </p>
        </div>
        {activeProject && (
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={() => undo()} disabled={pastStates.length === 0}>
              <Undo2 className="h-4 w-4 mr-2"/> Undo
            </Button>
            <Button variant="outline" size="sm" onClick={() => redo()} disabled={futureStates.length === 0}>
              <Redo2 className="h-4 w-4 mr-2"/> Redo
            </Button>
          </div>
        )}
      </div>
      
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
                <VideoPreview 
                    ref={videoPreviewRef} 
                    path={sourceData.path} 
                    editPlan={activeProject.editPlan} 
                    captionCues={activeProject.captionCues}
                    captionStyle={activeProject.captions}
                />
                <ExportPanel />
            </div>
            
            <div className="flex-1 w-full max-w-md h-[600px]">
                <EditReviewPanel mediaId={sourceData.id} onPreview={handlePreviewSuggestion} />
            </div>
        </div>
      )}
    </div>
  );
}
