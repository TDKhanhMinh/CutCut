import { useCallback, useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import {
  cancelMediaJob,
  exportPrototypeVideo,
  listenToMediaJobs,
  type MediaJobEvent,
} from "@/services/media";
import { useProjectStore } from "@/stores/useProjectStore";

export function ExportPanel() {
  const activeProject = useProjectStore((state) => state.activeProject);
  const [exporting, setExporting] = useState(false);
  const [progress, setProgress] = useState(0);
  const [jobId, setJobId] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const jobIdRef = useRef<string | null>(null);
  const bufferedEventsRef = useRef<MediaJobEvent[]>([]);

  const processMediaJobEvent = useCallback((event: MediaJobEvent) => {
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
  }, []);

  useEffect(() => {
    let active = true;
    const unlistenPromise = listenToMediaJobs((event) => {
      if (!active) return;

      if (event.jobId === jobIdRef.current) {
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
    if (!jobId) return;
    try {
      await cancelMediaJob(jobId);
    } catch (e) {
      console.error("Failed to cancel job:", e);
    }
  };

  return (
    <div className="m-4 max-w-xl rounded-lg border bg-card p-4 text-card-foreground shadow-sm">
      <h3 className="mb-4 text-lg font-bold">Export Prototype</h3>

      {!exporting ? (
        <Button onClick={handleExport}>Export to MP4</Button>
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
    </div>
  );
}
