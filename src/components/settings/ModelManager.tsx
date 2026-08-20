import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { BrainCircuit, CheckCircle2, Download, PlayCircle, Trash2, XCircle } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "../ui/dialog";
import { Button } from "../ui/button";
import { Progress } from "../ui/progress";
import type { ResourceItem, ResourceState } from "@/types/resource";

export function ModelManager() {
  const [models, setModels] = useState<ResourceItem[]>([]);
  const [states, setStates] = useState<Record<string, ResourceState>>({});
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [diskUsage, setDiskUsage] = useState(0);
  const [isOpen, setIsOpen] = useState(false);

  const fetchModels = async () => {
    try {
      const catalog = await invoke<ResourceItem[]>("get_models");
      setModels(catalog);

      const statesObject: Record<string, ResourceState> = {};
      for (const model of catalog) {
        statesObject[model.id] = await invoke<ResourceState>("get_model_state", { id: model.id });
      }
      setStates(statesObject);
      setActiveModel(await invoke<string | null>("get_active_model"));
      setDiskUsage(await invoke<number>("get_resource_usage"));
    } catch (error) {
      console.error("Failed to load models:", error);
    }
  };

  useEffect(() => {
    const unlistenProgress = listen<{
      id: string;
      progress: number;
      downloaded: number;
      total: number;
    }>("resource-download-progress", (event) => {
      const payload = event.payload;
      setStates((previous) => ({
        ...previous,
        [payload.id]: {
          downloading: {
            progress: payload.progress,
            downloaded: payload.downloaded,
            total: payload.total,
          },
        },
      }));
    });
    const unlistenFinished = listen<{
      id: string;
      status: "installed" | "cancelled" | "failed";
      reason?: string;
    }>("resource-download-finished", (event) => {
      if (event.payload.status === "failed") {
        setStates((previous) => ({
          ...previous,
          [event.payload.id]: {
            failed: { reason: event.payload.reason ?? "Tải resource thất bại." },
          },
        }));
        return;
      }
      void fetchModels();
    });

    return () => {
      unlistenProgress.then((dispose) => dispose());
      unlistenFinished.then((dispose) => dispose());
    };
  }, []);

  const handleOpenChange = (open: boolean) => {
    setIsOpen(open);
    if (open) void fetchModels();
  };

  const handleDownload = async (id: string) => {
    try {
      await invoke("download_model", { id });
    } catch (error) {
      console.error("Model download failed:", error);
      setStates((previous) => ({
        ...previous,
        [id]: { failed: { reason: String(error) } },
      }));
    }
  };

  const handleCancel = async (id: string) => {
    try {
      await invoke("cancel_model_download", { id });
    } catch (error) {
      console.error("Model download cancellation failed:", error);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("delete_model", { id });
      await fetchModels();
    } catch (error) {
      console.error("Model deletion failed:", error);
    }
  };

  const handleSetActive = async (id: string) => {
    try {
      await invoke("set_active_model", { id });
      setActiveModel(id);
    } catch (error) {
      console.error("Failed to select model:", error);
    }
  };

  const formatSize = (bytes: number) => `${(bytes / 1024 / 1024).toFixed(1)} MB`;

  return (
    <Dialog open={isOpen} onOpenChange={handleOpenChange}>
      <DialogTrigger render={<Button variant="outline" size="sm" className="gap-2" />}>
        <BrainCircuit className="h-4 w-4 text-primary" />
        <span className="hidden sm:inline">AI Models</span>
      </DialogTrigger>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Quản lý Speech Models</DialogTitle>
          <DialogDescription>
            Model chỉ được chọn sau khi checksum và compatibility với runtime CPU pass.
          </DialogDescription>
        </DialogHeader>

        <p className="text-xs text-muted-foreground">
          Đang dùng {formatSize(diskUsage)} trong app data directory.
        </p>
        <div className="mt-4 flex flex-col gap-4">
          {models.map((model) => {
            const state = states[model.id] ?? "notInstalled";
            const isInstalled = state === "installed";
            const isDownloading = typeof state === "object" && "downloading" in state;
            const isIncompatible = typeof state === "object" && "incompatible" in state;
            const isCorrupted = typeof state === "object" && "corrupted" in state;
            const isFailed = typeof state === "object" && "failed" in state;
            const progress = isDownloading ? state.downloading.progress * 100 : 0;
            const isActive = activeModel === model.id;

            return (
              <div
                key={model.id}
                className={`flex flex-col gap-3 rounded-lg border p-4 ${
                  isActive ? "border-primary bg-primary/5" : "bg-card"
                }`}
              >
                <div className="flex items-center justify-between gap-3">
                  <div className="flex min-w-0 flex-col">
                    <span className="flex items-center gap-2 font-medium">
                      {model.name}
                      {isActive && <CheckCircle2 className="h-4 w-4 text-primary" />}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {formatSize(model.sizeBytes)} · {model.id}
                    </span>
                  </div>

                  <div className="flex shrink-0 items-center gap-2">
                    {!isInstalled && !isDownloading && (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => void handleDownload(model.id)}
                      >
                        <Download className="h-4 w-4" />
                        {isCorrupted || isFailed ? "Tải lại" : "Tải về"}
                      </Button>
                    )}
                    {isDownloading && (
                      <Button
                        size="icon"
                        variant="ghost"
                        onClick={() => void handleCancel(model.id)}
                      >
                        <XCircle className="h-4 w-4" />
                      </Button>
                    )}
                    {isInstalled && !isActive && (
                      <Button size="sm" onClick={() => void handleSetActive(model.id)}>
                        <PlayCircle className="h-4 w-4" />
                        Dùng
                      </Button>
                    )}
                    {isInstalled && !isActive && (
                      <Button
                        size="icon"
                        variant="ghost"
                        className="text-destructive"
                        onClick={() => void handleDelete(model.id)}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    )}
                  </div>
                </div>

                {isDownloading && (
                  <div className="flex flex-col gap-1.5">
                    <div className="flex justify-between text-xs text-muted-foreground">
                      <span>Đang tải...</span>
                      <span>{progress.toFixed(0)}%</span>
                    </div>
                    <Progress value={progress} className="h-1.5" />
                  </div>
                )}
                {isIncompatible && (
                  <p className="text-xs text-amber-600">
                    Không tương thích: {state.incompatible.reason}
                  </p>
                )}
                {isCorrupted && (
                  <p className="text-xs text-destructive">
                    Checksum/manifest lỗi: {state.corrupted.reason} Hãy tải lại model.
                  </p>
                )}
                {isFailed && (
                  <p className="text-xs text-destructive">
                    Tải model thất bại: {state.failed.reason}
                  </p>
                )}
              </div>
            );
          })}
        </div>
      </DialogContent>
    </Dialog>
  );
}
