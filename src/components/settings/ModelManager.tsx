import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { BrainCircuit, Download, Trash2, CheckCircle2, PlayCircle } from "lucide-react";
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

type ResourceState =
  | "NotInstalled"
  | "Installed"
  | "Corrupted"
  | { Downloading: { progress: number; downloaded: number; total: number } };

interface ResourceItem {
  id: string;
  name: string;
  version: string;
  size_bytes: number;
  url: string;
}

export function ModelManager() {
  const [models, setModels] = useState<ResourceItem[]>([]);
  const [states, setStates] = useState<Record<string, ResourceState>>({});
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [isOpen, setIsOpen] = useState(false);

  const fetchModels = async () => {
    try {
      const catalog: ResourceItem[] = await invoke("get_models");
      setModels(catalog);

      const statesObj: Record<string, ResourceState> = {};
      for (const m of catalog) {
        const state: ResourceState = await invoke("get_model_state", { item: m });
        statesObj[m.id] = state;
      }
      setStates(statesObj);

      const active: string | null = await invoke("get_active_model");
      setActiveModel(active);
    } catch (error) {
      console.error("Failed to load models:", error);
    }
  };

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    if (isOpen) {
      void fetchModels();
    }
  }, [isOpen]);

  useEffect(() => {
    const unlistenProgress = listen("resource-download-progress", (event: { payload: { id: string; progress: number; downloaded: number; total: number } }) => {
      const payload = event.payload;
      setStates((prev) => ({
        ...prev,
        [payload.id]: {
          Downloading: {
            progress: payload.progress,
            downloaded: payload.downloaded,
            total: payload.total,
          },
        },
      }));
    });

    const unlistenFinished = listen("resource-download-finished", () => {
      fetchModels(); // Refresh states
    });

    return () => {
      unlistenProgress.then((f) => f());
      unlistenFinished.then((f) => f());
    };
  }, []);

  const handleDownload = async (id: string) => {
    try {
      // Optimistic UI for immediate feedback
      setStates((prev) => ({
        ...prev,
        [id]: { Downloading: { progress: 0, downloaded: 0, total: 100 } },
      }));
      await invoke("download_model", { id });
      await fetchModels();
    } catch (e) {
      console.error(e);
      await fetchModels();
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("delete_model", { id });
      if (activeModel === id) {
        await invoke("set_active_model", { id: "" });
      }
      await fetchModels();
    } catch (e) {
      console.error(e);
    }
  };

  const handleSetActive = async (id: string) => {
    try {
      await invoke("set_active_model", { id });
      setActiveModel(id);
    } catch (e) {
      console.error(e);
    }
  };

  const formatSize = (bytes: number) => {
    return (bytes / 1024 / 1024).toFixed(1) + " MB";
  };

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger 
        render={
          <Button variant="outline" size="sm" className="gap-2" />
        }
      >
        <BrainCircuit className="h-4 w-4 text-primary" />
        <span className="hidden sm:inline">AI Models</span>
      </DialogTrigger>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Quản lý Speech Models</DialogTitle>
          <DialogDescription>
            Tải và chọn model STT phù hợp với cấu hình máy của bạn.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4 mt-4">
          {models.map((m) => {
            const state = states[m.id] || "NotInstalled";
            const isInstalled = state === "Installed";
            const isDownloading = typeof state === "object" && "Downloading" in state;
            const progress = isDownloading ? (state.Downloading.progress * 100) : 0;
            const isActive = activeModel === m.id;

            return (
              <div
                key={m.id}
                className={`flex flex-col gap-3 rounded-lg border p-4 ${
                  isActive ? "border-primary bg-primary/5" : "bg-card"
                }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex flex-col">
                    <span className="font-medium flex items-center gap-2">
                      {m.name}
                      {isActive && (
                        <CheckCircle2 className="h-4 w-4 text-primary" />
                      )}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      Kích thước: {formatSize(m.size_bytes)}
                    </span>
                  </div>

                  <div className="flex items-center gap-2">
                    {!isInstalled && !isDownloading && (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => handleDownload(m.id)}
                        className="gap-2"
                      >
                        <Download className="h-4 w-4" />
                        Tải về
                      </Button>
                    )}
                    {isInstalled && !isActive && (
                      <Button
                        size="sm"
                        onClick={() => handleSetActive(m.id)}
                        className="gap-2"
                      >
                        <PlayCircle className="h-4 w-4" />
                        Dùng
                      </Button>
                    )}
                    {isInstalled && (
                      <Button
                        size="icon"
                        variant="ghost"
                        className="text-destructive"
                        onClick={() => handleDelete(m.id)}
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
              </div>
            );
          })}
        </div>
      </DialogContent>
    </Dialog>
  );
}
