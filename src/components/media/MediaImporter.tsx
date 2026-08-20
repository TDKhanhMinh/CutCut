import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { readMediaMetadata } from "@/services/media";
import type { MediaSourceMetadata } from "@/types/media";

export function MediaImporter({
  onMetadataParsed,
}: {
  onMetadataParsed: (meta: MediaSourceMetadata) => void;
}) {
  const [metadata, setMetadata] = useState<MediaSourceMetadata | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSelectFile = async () => {
    try {
      setError(null);
      setMetadata(null);

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

      // Path inside selected is absolute, we pass it to backend
      const result = await readMediaMetadata(filePath);
      setMetadata(result);
      onMetadataParsed(result);
    } catch (e: unknown) {
      console.error("Failed to read metadata:", e);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="m-4 max-w-xl rounded-lg border bg-card p-4 text-card-foreground shadow-sm">
      <h2 className="mb-4 text-xl font-bold">Media Importer (Task 7 Test)</h2>
      <Button onClick={handleSelectFile} disabled={loading}>
        {loading ? "Reading metadata..." : "Select Video File"}
      </Button>

      {error && (
        <div className="mt-4 rounded border border-destructive bg-destructive/20 p-3 text-sm text-destructive">
          <strong>Error:</strong> {error}
        </div>
      )}

      {metadata && (
        <div className="mt-4">
          <h3 className="mb-2 font-semibold">Metadata Parsed:</h3>
          <pre className="overflow-x-auto rounded bg-muted p-3 text-sm">
            {JSON.stringify(metadata, null, 2)}
          </pre>
        </div>
      )}
    </div>
  );
}
