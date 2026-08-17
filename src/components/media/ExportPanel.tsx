import { useState, useEffect } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

interface ExportPanelProps {
    inputPath: string;
    totalDurationSec: number;
}

export function ExportPanel({ inputPath, totalDurationSec }: ExportPanelProps) {
    const [exporting, setExporting] = useState(false);
    const [progress, setProgress] = useState(0);
    const [jobId, setJobId] = useState<string | null>(null);
    const [result, setResult] = useState<string | null>(null);

    useEffect(() => {
        let unlisten: UnlistenFn | null = null;

        const setupListener = async () => {
            unlisten = await listen<{ jobId: string; eventType: string; progress?: number; error?: string }>('media-job-progress', (event) => {
                if (event.payload.jobId === jobId) {
                    if (event.payload.eventType === 'Progress' && event.payload.progress !== undefined) {
                        setProgress(event.payload.progress);
                    } else if (event.payload.eventType === 'Completed') {
                        setExporting(false);
                        setResult('Export completed successfully!');
                    } else if (event.payload.eventType === 'Failed') {
                        setExporting(false);
                        setResult(`Export failed: ${event.payload.error}`);
                    } else if (event.payload.eventType === 'Cancelled') {
                        setExporting(false);
                        setResult('Export was cancelled.');
                    }
                }
            });
        };

        if (jobId) {
            setupListener();
        }

        return () => {
            if (unlisten) unlisten();
        };
    }, [jobId]);

    const handleExport = async () => {
        try {
            const outputPath = await save({
                filters: [{ name: 'MP4 Video', extensions: ['mp4'] }],
                defaultPath: 'prototype_export.mp4'
            });

            if (!outputPath) return; // User cancelled dialog

            setResult(null);
            setProgress(0);
            setExporting(true);

            const newJobId = await invoke<string>('export_prototype_video', {
                inputPath,
                outputPath,
                totalDurationSec
            });

            setJobId(newJobId);
        } catch (e: any) {
            console.error(e);
            setExporting(false);
            setResult(`Failed to start export: ${e.toString()}`);
        }
    };

    const handleCancel = async () => {
        if (!jobId) return;
        try {
            await invoke('cancel_media_job', { jobId });
        } catch (e) {
            console.error('Failed to cancel job:', e);
        }
    };

    return (
        <div className="p-4 border rounded-lg bg-card text-card-foreground shadow-sm m-4 max-w-xl">
            <h3 className="text-lg font-bold mb-4">Export Prototype</h3>
            
            {!exporting ? (
                <button
                    className="bg-primary text-primary-foreground px-4 py-2 rounded-md font-medium hover:bg-primary/90"
                    onClick={handleExport}
                >
                    Export to MP4
                </button>
            ) : (
                <div className="space-y-4">
                    <div className="flex justify-between items-center text-sm font-medium">
                        <span>Exporting...</span>
                        <span>{Math.round(progress * 100)}%</span>
                    </div>
                    <div className="w-full bg-secondary rounded-full h-2.5">
                        <div 
                            className="bg-primary h-2.5 rounded-full transition-all duration-300" 
                            style={{ width: `${Math.max(0, Math.min(100, progress * 100))}%` }}
                        ></div>
                    </div>
                    <button
                        className="bg-destructive text-destructive-foreground px-4 py-2 rounded-md font-medium hover:bg-destructive/90"
                        onClick={handleCancel}
                    >
                        Cancel
                    </button>
                </div>
            )}

            {result && (
                <div className="mt-4 p-3 bg-muted rounded text-sm font-medium">
                    {result}
                </div>
            )}
        </div>
    );
}
