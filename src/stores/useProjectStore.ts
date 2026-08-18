import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';
import { Project } from '../types/project';

interface ProjectState {
    activeProject: Project | null;
    projectPath: string | null;
    isDirty: boolean;
    saveState: 'idle' | 'saving' | 'saved' | 'error';
    
    // Actions
    setProject: (project: Project, path: string | null) => void;
    updateProject: (updater: (draft: Project) => void) => void; // A simple mutator, we'll clone deep enough for now
    
    // Commands
    saveProject: () => Promise<void>;
    setSaveState: (state: 'idle' | 'saving' | 'saved' | 'error') => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
    activeProject: null,
    projectPath: null,
    isDirty: false,
    saveState: 'idle',

    setProject: (project, path) => set({ 
        activeProject: project, 
        projectPath: path, 
        isDirty: false, 
        saveState: 'idle' 
    }),

    updateProject: (updater) => {
        set((state) => {
            if (!state.activeProject) return state;
            
            // Shallow clone for React reactivity
            const newProject = { ...state.activeProject };
            // Optional: for arrays, we'd need deeper clone if modifying
            newProject.media = [...newProject.media];
            newProject.settings = { ...newProject.settings };
            
            updater(newProject);
            
            return {
                activeProject: newProject,
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
    }
}));
