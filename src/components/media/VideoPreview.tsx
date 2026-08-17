import { convertFileSrc } from '@tauri-apps/api/core';

interface VideoPreviewProps {
    path: string;
}

export function VideoPreview({ path }: VideoPreviewProps) {
    // Convert absolute local path to Tauri asset protocol URL
    const assetUrl = convertFileSrc(path);

    return (
        <div className="p-4 border rounded-lg bg-card text-card-foreground shadow-sm m-4 max-w-xl">
            <h3 className="text-lg font-bold mb-2">Video Preview</h3>
            <video 
                src={assetUrl} 
                controls 
                className="w-full bg-black rounded"
                style={{ maxHeight: '400px' }}
            />
        </div>
    );
}
