import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';

export interface MediaSourceMetadata {
    path: string;
    durationSec: number;
    fps: number;
    width: number;
    height: number;
    videoCodec: string;
    audioCodec?: string;
    rotation: number;
}

export function MediaImporter() {
    const [metadata, setMetadata] = useState<MediaSourceMetadata | null>(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const handleSelectFile = async () => {
        try {
            setError(null);
            setMetadata(null);
            
            const selected = await open({
                multiple: false,
                filters: [{
                    name: 'Video',
                    extensions: ['mp4', 'mov', 'mkv', 'webm', 'avi']
                }]
            });

            if (selected === null) {
                return; // User cancelled
            }

            const filePath = Array.isArray(selected) ? selected[0] : selected;
            setLoading(true);

            // Path inside selected is absolute, we pass it to backend
            const result = await invoke<MediaSourceMetadata>('read_media_metadata', { path: filePath });
            setMetadata(result);
        } catch (e: any) {
            console.error('Failed to read metadata:', e);
            setError(e.toString());
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className="p-4 border rounded-lg bg-card text-card-foreground shadow-sm m-4 max-w-xl">
            <h2 className="text-xl font-bold mb-4">Media Importer (Task 7 Test)</h2>
            <button
                className="bg-primary text-primary-foreground px-4 py-2 rounded-md font-medium hover:bg-primary/90 disabled:opacity-50"
                onClick={handleSelectFile}
                disabled={loading}
            >
                {loading ? 'Reading metadata...' : 'Select Video File'}
            </button>

            {error && (
                <div className="mt-4 p-3 bg-destructive/20 text-destructive border border-destructive rounded text-sm">
                    <strong>Error:</strong> {error}
                </div>
            )}

            {metadata && (
                <div className="mt-4">
                    <h3 className="font-semibold mb-2">Metadata Parsed:</h3>
                    <pre className="bg-muted p-3 rounded text-sm overflow-x-auto">
                        {JSON.stringify(metadata, null, 2)}
                    </pre>
                </div>
            )}
        </div>
    );
}
