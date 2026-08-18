import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { AlertTriangle } from 'lucide-react';
import { useProjectStore } from '../../stores/useProjectStore';
import { MediaSourceMetadata } from './MediaImporter';

export function MediaRelink({ mediaId, oldPath }: { mediaId: string, oldPath: string }) {
    const relinkMedia = useProjectStore((state) => state.relinkMedia);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const handleRelink = async () => {
        try {
            setError(null);
            
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

            // Fetch new metadata
            const newMetadata = await invoke<MediaSourceMetadata>('read_media_metadata', { path: filePath });
            
            // Relink the media in store
            relinkMedia(mediaId, filePath, newMetadata);
            
        } catch (e: unknown) {
            console.error('Failed to relink media:', e);
            setError(e instanceof Error ? e.message : String(e));
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className="flex flex-col items-center justify-center p-8 bg-destructive/10 border border-destructive/20 rounded-xl text-center max-w-md w-full mx-auto my-8">
            <div className="w-12 h-12 rounded-full bg-destructive/20 flex items-center justify-center mb-4 text-destructive">
                <AlertTriangle size={24} />
            </div>
            
            <h2 className="text-xl font-bold mb-2">Media File Missing</h2>
            
            <p className="text-muted-foreground text-sm mb-2">
                We can't find the source media for this project at its original location:
            </p>
            
            <div className="bg-background p-2 rounded text-xs font-mono break-all text-muted-foreground mb-6 w-full text-left">
                {oldPath}
            </div>
            
            <p className="text-sm mb-6">
                Your project edits are safe, but you need to relink the media file to continue editing or exporting.
            </p>
            
            {error && (
                <div className="mb-4 p-3 bg-destructive/20 text-destructive border border-destructive rounded text-sm w-full text-left">
                    <strong>Error:</strong> {error}
                </div>
            )}
            
            <button
                className="bg-primary text-primary-foreground px-6 py-2 rounded-md font-medium hover:bg-primary/90 disabled:opacity-50"
                onClick={handleRelink}
                disabled={loading}
            >
                {loading ? 'Relinking...' : 'Relink File'}
            </button>
        </div>
    );
}
