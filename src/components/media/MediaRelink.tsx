import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle } from "lucide-react";
import { useProjectStore } from "../../stores/useProjectStore";
import {
  compareMediaMetadata,
  readMediaMetadata,
  type MediaCompatibilityResult,
} from "@/services/media";
import type { MediaSourceMetadata } from "@/types/media";

interface PendingRelink {
  path: string;
  metadata: MediaSourceMetadata;
}

interface MediaRelinkProps {
  mediaId: string;
  oldPath: string;
  oldMetadata: MediaSourceMetadata;
}

export function MediaRelink({ mediaId, oldPath, oldMetadata }: MediaRelinkProps) {
  const relinkMedia = useProjectStore((state) => state.relinkMedia);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [compatibility, setCompatibility] = useState<MediaCompatibilityResult | null>(null);
  const [pendingRelink, setPendingRelink] = useState<PendingRelink | null>(null);

  const confirmRelink = () => {
    if (!pendingRelink) return;

    relinkMedia(mediaId, pendingRelink.path, pendingRelink.metadata);
    setPendingRelink(null);
    setCompatibility(null);
  };

  const handleRelink = async () => {
    try {
      setError(null);

      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Video",
            extensions: ["mp4", "mov", "mkv", "webm", "avi"],
          },
        ],
      });

      if (selected === null) {
        return; // User cancelled
      }

      const filePath = Array.isArray(selected) ? selected[0] : selected;
      setLoading(true);

      // Fetch new metadata
      const newMetadata = await readMediaMetadata(filePath);
      const result = compareMediaMetadata(oldMetadata, newMetadata);

      if (result.requiresConfirmation) {
        setCompatibility(result);
        setPendingRelink({ path: filePath, metadata: newMetadata });
        return;
      }

      // Relink the media in store
      relinkMedia(mediaId, filePath, newMetadata);
    } catch (e: unknown) {
      console.error("Failed to relink media:", e);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  const discardPendingRelink = () => {
    setPendingRelink(null);
    setCompatibility(null);
  };

  return (
    <div className="mx-auto my-8 flex w-full max-w-md flex-col items-center justify-center rounded-xl border border-destructive/20 bg-destructive/10 p-8 text-center">
      <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-destructive/20 text-destructive">
        <AlertTriangle size={24} />
      </div>

      <h2 className="mb-2 text-xl font-bold">Media File Missing</h2>

      <p className="mb-2 text-sm text-muted-foreground">
        We can't find the source media for this project at its original location:
      </p>

      <div className="mb-6 w-full break-all rounded bg-background p-2 text-left font-mono text-xs text-muted-foreground">
        {oldPath}
      </div>

      <p className="mb-6 text-sm">
        Your project edits are safe, but you need to relink the media file to continue editing or
        exporting.
      </p>

      {error && (
        <div className="mb-4 w-full rounded border border-destructive bg-destructive/20 p-3 text-left text-sm text-destructive">
          <strong>Error:</strong> {error}
        </div>
      )}

      {compatibility && pendingRelink && (
        <div
          className="mb-4 w-full rounded border border-yellow-500/40 bg-yellow-500/10 p-3 text-left text-sm"
          role="alert"
        >
          <p className="mb-2 font-semibold text-yellow-700 dark:text-yellow-300">
            Media thay thế không hoàn toàn tương thích
          </p>
          <ul className="mb-3 list-inside list-disc space-y-1 text-muted-foreground">
            {compatibility.warnings.map((warning) => (
              <li key={warning.code}>{warning.message}</li>
            ))}
          </ul>
          <p className="mb-3 text-xs text-muted-foreground">
            Transcript và Edit Plan vẫn giữ timestamp của source cũ. Chỉ tiếp tục nếu bạn xác nhận
            timeline có thể thay đổi.
          </p>
          <div className="flex flex-wrap gap-2">
            <button
              className="rounded-md bg-yellow-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-yellow-700"
              onClick={confirmRelink}
              type="button"
            >
              Relink với cảnh báo
            </button>
            <button
              className="rounded-md border border-border px-3 py-1.5 text-xs font-medium hover:bg-muted"
              onClick={discardPendingRelink}
              type="button"
            >
              Chọn file khác
            </button>
          </div>
        </div>
      )}

      <button
        className="rounded-md bg-primary px-6 py-2 font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
        onClick={handleRelink}
        disabled={loading}
      >
        {loading ? "Relinking..." : pendingRelink ? "Chọn file khác" : "Relink File"}
      </button>
    </div>
  );
}
