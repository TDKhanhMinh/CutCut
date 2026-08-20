import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle } from "lucide-react";
import { useProjectStore } from "../../stores/useProjectStore";
import { readMediaMetadata } from "@/services/media";

export function MediaRelink({ mediaId, oldPath }: { mediaId: string; oldPath: string }) {
  const relinkMedia = useProjectStore((state) => state.relinkMedia);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

      // Relink the media in store
      relinkMedia(mediaId, filePath, newMetadata);
    } catch (e: unknown) {
      console.error("Failed to relink media:", e);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
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

      <button
        className="rounded-md bg-primary px-6 py-2 font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
        onClick={handleRelink}
        disabled={loading}
      >
        {loading ? "Relinking..." : "Relink File"}
      </button>
    </div>
  );
}
