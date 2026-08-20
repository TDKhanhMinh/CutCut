import { convertFileSrc } from "@tauri-apps/api/core";

interface VideoPreviewProps {
  path: string;
}

export function VideoPreview({ path }: VideoPreviewProps) {
  // Convert absolute local path to Tauri asset protocol URL
  const assetUrl = convertFileSrc(path);

  return (
    <div className="m-4 max-w-xl rounded-lg border bg-card p-4 text-card-foreground shadow-sm">
      <h3 className="mb-2 text-lg font-bold">Video Preview</h3>
      <video
        src={assetUrl}
        controls
        className="w-full rounded bg-black"
        style={{ maxHeight: "400px" }}
      />
    </div>
  );
}
