import { create } from "zustand";
import { Project } from "../types/project";
import { saveProjectToDisk } from "@/services/project";

interface ProjectState {
  activeProject: Project | null;
  projectPath: string | null;
  isDirty: boolean;
  revision: number;
  saveState: "idle" | "saving" | "saved" | "error";
  lastSaveError: string | null;

  // Actions
  setProject: (project: Project, path: string | null) => void;
  updateProject: (updater: (draft: Project) => void) => void; // A simple mutator, we'll clone deep enough for now

  // Commands
  saveProject: () => Promise<void>;
  saveProjectAs: (path: string) => Promise<void>;
  setSaveState: (state: "idle" | "saving" | "saved" | "error") => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  activeProject: null,
  projectPath: null,
  isDirty: false,
  revision: 0,
  saveState: "idle",
  lastSaveError: null,

  setProject: (project, path) =>
    set((state) => ({
      activeProject: project,
      projectPath: path,
      isDirty: false,
      saveState: "idle",
      lastSaveError: null,
      revision: state.revision + 1,
    })),

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
        saveState: "idle",
        revision: state.revision + 1,
      };
    });
  },

  setSaveState: (saveState) => set({ saveState }),

  saveProject: async () => {
    const { activeProject, projectPath, revision } = get();
    if (!activeProject || !projectPath) return;

    set({ saveState: "saving", lastSaveError: null });
    try {
      await saveProjectToDisk(projectPath, activeProject);
      if (get().revision === revision) {
        set({ saveState: "saved", isDirty: false });
      }
    } catch (e) {
      console.error("Failed to save project:", e);
      set({
        saveState: "error",
        lastSaveError: e instanceof Error ? e.message : String(e),
      });
    }
  },

  saveProjectAs: async (path) => {
    const { activeProject, revision } = get();
    if (!activeProject) return;

    set({ saveState: "saving", lastSaveError: null });
    try {
      await saveProjectToDisk(path, activeProject);
      if (get().revision === revision) {
        set({ projectPath: path, saveState: "saved", isDirty: false });
      }
    } catch (e) {
      console.error("Failed to save project as:", e);
      set({
        saveState: "error",
        lastSaveError: e instanceof Error ? e.message : String(e),
      });
    }
  },
}));
