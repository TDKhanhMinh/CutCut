import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  cancelMediaJob,
  exportPrototypeVideo,
  finalizePreviewArtifact,
  previewPrototypeVideo,
  listenToMediaJobs,
  type MediaJobEvent,
} from "@/services/media";
import { useProjectStore } from "@/stores/useProjectStore";

interface ExportPanelProps {
  /** Source-time playhead used as the default Accurate Preview range start. */
  previewStartMs?: number;
}

export function ExportPanel({ previewStartMs = 0 }: ExportPanelProps) {
  const activeProject = useProjectStore((state) => state.activeProject);
  const [exporting, setExporting] = useState(false);
  const [progress, setProgress] = useState(0);
  const [jobId, setJobId] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [previewProgress, setPreviewProgress] = useState(0);
  const [previewPath, setPreviewPath] = useState<string | null>(null);
  const [previewArtifact, setPreviewArtifact] = useState<
    import("@/types/artifact").ArtifactRecord | null
  >(null);
  const [rangeStartMs, setRangeStartMs] = useState(() => Math.max(0, Math.round(previewStartMs)));

  const jobIdRef = useRef<string | null>(null);
  const previewJobIdRef = useRef<string | null>(null);
  const bufferedEventsRef = useRef<MediaJobEvent[]>([]);

  const processMediaJobEvent = useCallback(
    (event: MediaJobEvent) => {
      if (event.jobId === previewJobIdRef.current) {
        if (event.state === "progress" && event.progress !== undefined) {
          setPreviewProgress(event.progress);
        } else if (event.state === "completed") {
          setPreviewProgress(1);
          setPreviewing(false);
          const currentProject = useProjectStore.getState().activeProject;
          const artifact = previewArtifact;
          if (currentProject && artifact) {
            void finalizePreviewArtifact(currentProject, artifact)
              .then((validArtifact) => {
                useProjectStore.getState().updateProject((draft) => {
                  draft.artifacts = [
                    ...draft.artifacts.filter((candidate) => candidate.id !== validArtifact.id),
                    validArtifact,
                  ];
                });
                setPreviewArtifact(validArtifact);
              })
              .catch((error: unknown) =>
                setResult(`Preview artifact validation failed: ${String(error)}`),
              );
          }
        } else if (event.state === "failed" || event.state === "cancelled") {
          setPreviewing(false);
          setResult(
            event.state === "failed"
              ? `Preview failed: ${event.error ?? "Unknown error"}`
              : "Preview was cancelled.",
          );
        }
        return;
      }

      if (event.state === "progress" && event.progress !== undefined) {
        setProgress(event.progress);
      } else if (event.state === "completed") {
        setProgress(1);
        setExporting(false);
        setResult("Export completed successfully!");
      } else if (event.state === "failed") {
        setExporting(false);
        setResult(`Export failed: ${event.error ?? "Unknown error"}`);
      } else if (event.state === "cancelled") {
        setExporting(false);
        setResult("Export was cancelled.");
      }
    },
    [previewArtifact],
  );

  useEffect(() => {
    let active = true;
    const unlistenPromise = listenToMediaJobs((event) => {
      if (!active) return;

      if (event.jobId === jobIdRef.current || event.jobId === previewJobIdRef.current) {
        processMediaJobEvent(event);
      } else if (jobIdRef.current === null) {
        bufferedEventsRef.current = [...bufferedEventsRef.current.slice(-19), event];
      }
    });

    return () => {
      active = false;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [processMediaJobEvent]);

  const handleExport = async () => {
    try {
      const outputPath = await save({
        filters: [{ name: "MP4 Video", extensions: ["mp4"] }],
        defaultPath: "prototype_export.mp4",
      });

      if (!outputPath) return; // User cancelled dialog

      setResult(null);
      setProgress(0);
      setExporting(true);
      jobIdRef.current = null;
      setJobId(null);
      bufferedEventsRef.current = [];

      if (!activeProject) throw new Error("No active project");
      const newJobId = await exportPrototypeVideo(activeProject, outputPath);

      jobIdRef.current = newJobId;
      setJobId(newJobId);
      const bufferedEvents = bufferedEventsRef.current.filter((event) => event.jobId === newJobId);
      bufferedEventsRef.current = [];
      bufferedEvents.forEach(processMediaJobEvent);
    } catch (e: unknown) {
      console.error(e);
      setExporting(false);
      setResult(`Failed to start export: ${e instanceof Error ? e.message : String(e)}`);
    }
  };

  const handleCancel = async () => {
    const activeJobId = jobId ?? previewJobIdRef.current;
    if (!activeJobId) return;
    try {
      await cancelMediaJob(activeJobId);
      if (activeJobId === previewJobIdRef.current) setPreviewing(false);
    } catch (e) {
      console.error("Failed to cancel job:", e);
    }
  };

  const handleAccuratePreview = async () => {
    if (!activeProject?.media[0]) return;
    const durationMs = Math.round(activeProject.media[0].metadata.durationSec * 1000);
    if (durationMs < 3_000) {
      setResult("Accurate Preview requires at least 3 seconds of source media.");
      return;
    }
    try {
      setResult(null);
      setPreviewPath(null);
      setPreviewProgress(0);
      setPreviewing(true);
      previewJobIdRef.current = null;
      const safeStartMs = Math.min(
        Math.max(0, Math.round(rangeStartMs)),
        Math.max(0, durationMs - 3_000),
      );
      const rangeDurationMs = Math.min(5_000, durationMs - safeStartMs);
      const response = await previewPrototypeVideo(
        activeProject,
        safeStartMs,
        safeStartMs + rangeDurationMs,
      );
      setPreviewArtifact(response.artifact);
      if (response.cachedPath) setPreviewPath(response.cachedPath);
      if (response.jobId) {
        previewJobIdRef.current = response.jobId;
        const bufferedEvents = bufferedEventsRef.current.filter(
          (event) => event.jobId === response.jobId,
        );
        bufferedEventsRef.current = bufferedEventsRef.current.filter(
          (event) => event.jobId !== response.jobId,
        );
        bufferedEvents.forEach(processMediaJobEvent);
      } else {
        setPreviewProgress(1);
        setPreviewing(false);
      }
      if (response.artifact) {
        useProjectStore.getState().updateProject((draft) => {
          draft.artifacts = [
            ...draft.artifacts.filter((candidate) => candidate.id !== response.artifact?.id),
            response.artifact!,
          ];
        });
      }
    } catch (error: unknown) {
      setPreviewing(false);
      setResult(`Failed to start accurate preview: ${String(error)}`);
    }
  };

  return (
    <div className="m-4 max-w-xl rounded-lg border bg-card p-4 text-card-foreground shadow-sm">
      <h3 className="mb-4 text-lg font-bold">Export Prototype</h3>

      {!exporting ? (
        <div className="space-y-3">
          <div className="grid grid-cols-[1fr_auto] items-center gap-2 text-sm">
            <label htmlFor="accurate-preview-start">Preview start (ms)</label>
            <Input
              id="accurate-preview-start"
              type="number"
              min={0}
              step={100}
              value={rangeStartMs}
              onChange={(event) => setRangeStartMs(Math.max(0, Number(event.target.value) || 0))}
              className="w-32"
              aria-describedby="accurate-preview-range-help"
            />
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="col-span-2 justify-self-end"
              onClick={() => setRangeStartMs(Math.max(0, Math.round(previewStartMs)))}
            >
              Use playhead
            </Button>
            <p
              id="accurate-preview-range-help"
              className="col-span-2 text-xs text-muted-foreground"
            >
              Accurate Preview tự clamp range còn 3–5 giây trong source timeline.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button onClick={handleExport}>Export to MP4</Button>
            <Button
              variant="secondary"
              onClick={handleAccuratePreview}
              disabled={previewing || !activeProject?.media.length}
            >
              {previewing
                ? `Rendering preview (${Math.round(previewProgress * 100)}%)`
                : "Accurate Preview (3–5s)"}
            </Button>
            {previewing && (
              <Button variant="destructive" onClick={handleCancel}>
                Cancel preview
              </Button>
            )}
          </div>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="flex items-center justify-between text-sm font-medium">
            <span>Exporting...</span>
            <span>{Math.round(progress * 100)}%</span>
          </div>
          <div className="h-2.5 w-full rounded-full bg-secondary">
            <div
              className="h-2.5 rounded-full bg-primary transition-all duration-300"
              style={{ width: `${Math.max(0, Math.min(100, progress * 100))}%` }}
            ></div>
          </div>
          <Button variant="destructive" onClick={handleCancel}>
            Cancel
          </Button>
        </div>
      )}

      {result && <div className="mt-4 rounded bg-muted p-3 text-sm font-medium">{result}</div>}
      {previewPath && !previewing && (
        <div className="mt-4 space-y-2">
          <div className="text-sm font-medium">Rendered preview</div>
          <video
            className="max-h-72 w-full rounded border"
            controls
            src={convertFileSrc(previewPath)}
          />
          {previewArtifact?.status === "valid" && (
            <div className="text-xs text-muted-foreground">Cached by render signature.</div>
          )}
        </div>
      )}
    </div>
  );
}
