import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { readMediaMetadata } from "@/services/media";
import type { MediaSourceMetadata } from "@/types/media";
import { useI18n } from "@/i18n";

export function MediaImporter({
  onMetadataParsed,
}: {
  onMetadataParsed: (meta: MediaSourceMetadata) => void;
}) {
  const { t } = useI18n();
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
      <h2 className="mb-4 text-xl font-bold">{t("media.importerTitle")}</h2>
      <Button onClick={handleSelectFile} disabled={loading}>
        {loading ? t("media.readingMetadata") : t("media.selectVideo")}
      </Button>

      {error && (
        <div className="mt-4 rounded border border-destructive bg-destructive/20 p-3 text-sm text-destructive">
          <strong>{t("common.error")}:</strong> {error}
        </div>
      )}

      {metadata && (
        <div className="mt-4">
          <h3 className="mb-2 font-semibold">{t("media.metadataParsed")}:</h3>
          <pre className="overflow-x-auto rounded bg-muted p-3 text-sm">
            {JSON.stringify(metadata, null, 2)}
          </pre>
        </div>
      )}
    </div>
  );
}
