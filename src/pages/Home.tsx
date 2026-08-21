import { useEffect, useRef, useState } from "react";
import { useStore } from "zustand";
import { Undo2, Redo2 } from "lucide-react";
import { useAuthStore } from "../stores/useAuthStore";
import { useEntitlementStore } from "../stores/useEntitlementStore";
import { AuthDialog } from "../components/editor/AuthDialog";
import { useProjectStore } from "../stores/useProjectStore";
import { MediaImporter } from "../components/media/MediaImporter";
import { VideoPreview, type VideoPreviewRef } from "../components/media/VideoPreview";
import { ExportPanel } from "../components/media/ExportPanel";
import { MediaRelink } from "../components/media/MediaRelink";
import { ReviewControls } from "../components/editor/ReviewControls";
import { EditReviewPanel } from "../components/editor/EditReviewPanel";
import { DEFAULT_SILENCE_CONFIG } from "@/types/silence";
import { Button } from "@/components/ui/button";

export function Home() {
  const { activeProject, updateProject, missingMediaIds } = useProjectStore();
  const { undo, redo, pastStates, futureStates } = useStore(
    useProjectStore.temporal,
    (state) => state,
  );
  const { user, isInitialized, initialize, signOut } = useAuthStore();
  const { plan, loading: entitlementLoading } = useEntitlementStore();
  const [authDialogOpen, setAuthDialogOpen] = useState(false);
  const [currentTimeMs, setCurrentTimeMs] = useState(0);
  const videoPreviewRef = useRef<VideoPreviewRef>(null);

  useEffect(() => {
    if (!isInitialized) void initialize();
  }, [isInitialized, initialize]);

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
      <AuthDialog open={authDialogOpen} onOpenChange={setAuthDialogOpen} />
      <div className="mb-4 flex items-start justify-between">
        <div>
          <h1 className="mb-1 text-2xl font-bold">CutCut Media Toolchain</h1>
          <p className="mb-8 text-muted-foreground">
            Prototype proving native file dialog, FFprobe metadata parsing, local video preview
            (Asset Protocol), and FFmpeg export jobs.
          </p>
        </div>
        <div className="flex flex-col items-end gap-2">
          <div className="flex items-center gap-2">
            {user ? (
              <span className="mr-2 text-sm font-medium text-green-500">
                {user.email} <span className="ml-1 rounded bg-green-500/20 px-2 py-0.5 text-xs">{entitlementLoading ? "..." : plan}</span>
              </span>
            ) : (
              <span className="mr-2 text-sm text-muted-foreground">Offline / Signed Out</span>
            )}
            {user ? (
              <Button variant="outline" size="sm" onClick={() => void signOut()}>Sign Out</Button>
            ) : (
              <Button variant="outline" size="sm" onClick={() => setAuthDialogOpen(true)}>Sign In</Button>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={() => undo()} disabled={pastStates.length === 0}>
              <Undo2 className="mr-2 h-4 w-4" /> Undo
            </Button>
            <Button variant="outline" size="sm" onClick={() => redo()} disabled={futureStates.length === 0}>
              <Redo2 className="mr-2 h-4 w-4" /> Redo
            </Button>
          </div>
        </div>
      </div>

      {!sourceData && (
        <MediaImporter
          onMetadataParsed={(meta) => {
            updateProject((draft) => {
              draft.media = [{ id: "demo-media-1", path: meta.path, metadata: meta }];
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
              onCurrentTimeChange={setCurrentTimeMs}
            />
            <ExportPanel previewStartMs={currentTimeMs} />
          </div>
          <div className="w-full max-w-md flex-1 space-y-4">
            <ReviewControls
              mediaId={sourceData.id}
              media={activeProject.media}
              sourcePath={sourceData.path}
              durationMs={Math.max(0, Math.round(sourceData.metadata.durationSec * 1000))}
              transcript={activeProject.transcript}
              silenceConfig={activeProject.silenceSettings ?? DEFAULT_SILENCE_CONFIG}
              editPlan={activeProject.editPlan}
              onEditPlanChange={handleEditPlanChange}
              onPreview={handlePreviewSuggestion}
            />
            <div className="h-[600px]">
              <EditReviewPanel mediaId={sourceData.id} onPreview={handlePreviewSuggestion} />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
