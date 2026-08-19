import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';
import { produce } from 'immer';
import { temporal } from 'zundo';
import { Project } from '../types/project';
import { MediaSourceMetadata } from '../components/media/MediaImporter';

interface ProjectState {
    activeProject: Project | null;
    projectPath: string | null;
    isDirty: boolean;
    saveState: 'idle' | 'saving' | 'saved' | 'error';
    missingMediaIds: string[];
    
    // Actions
    setProject: (project: Project, path: string | null) => void;
    updateProject: (updater: (draft: Project) => void) => void; 
    
    // Commands
    saveProject: () => Promise<void>;
    setSaveState: (state: 'idle' | 'saving' | 'saved' | 'error') => void;
    checkMediaStatus: () => Promise<void>;
    relinkMedia: (mediaId: string, newPath: string, newMetadata: MediaSourceMetadata) => void;
}

export const useProjectStore = create<ProjectState>()(
    temporal(
        (set, get) => ({
            activeProject: null,
            projectPath: null,
            isDirty: false,
            saveState: 'idle',
            missingMediaIds: [],

            setProject: (project, path) => {
                set({ 
                    activeProject: project, 
                    projectPath: path, 
                    isDirty: false, 
                    saveState: 'idle' 
                });
                get().checkMediaStatus();
            },

            updateProject: (updater) => {
                set((state) => {
                    if (!state.activeProject) return state;
                    
                    const nextProject = produce(state.activeProject, updater);
                    
                    return {
                        activeProject: nextProject,
                        isDirty: true,
                        saveState: 'idle',
                    };
                });
            },

            setSaveState: (saveState) => set({ saveState }),

            saveProject: async () => {
                const { activeProject, projectPath } = get();
                if (!activeProject || !projectPath) return;

                set({ saveState: 'saving' });
                try {
                    await invoke('save_project_to_disk', { 
                        path: projectPath, 
                        project: activeProject 
                    });
                    set({ saveState: 'saved', isDirty: false });
                } catch (e) {
                    console.error('Failed to save project:', e);
                    set({ saveState: 'error' });
                }
            },

            checkMediaStatus: async () => {
                const { activeProject } = get();
                if (!activeProject) return;

                const missingIds: string[] = [];
                for (const media of activeProject.media) {
                    try {
                        const exists = await invoke<boolean>('check_media_exists', { path: media.path });
                        if (!exists) {
                            missingIds.push(media.id);
                        }
                    } catch (e) {
                        console.error(`Failed to check media path ${media.path}:`, e);
                        missingIds.push(media.id);
                    }
                }
                set({ missingMediaIds: missingIds });
            },

            relinkMedia: (mediaId, newPath, newMetadata) => {
                get().updateProject((draft) => {
                    const mediaIndex = draft.media.findIndex(m => m.id === mediaId);
                    if (mediaIndex !== -1) {
                        draft.media[mediaIndex] = {
                            ...draft.media[mediaIndex],
                            path: newPath,
                            metadata: newMetadata,
                        };
                    }
                });
                
                // Remove from missing list
                set((state) => ({
                    missingMediaIds: state.missingMediaIds.filter(id => id !== mediaId)
                }));
            }
        }),
        {
            partialize: (state) => ({ activeProject: state.activeProject }),
            limit: 100, // Limit history to 100 steps
        }
    )
);
