import { create } from "zustand";
import { checkMediaExists } from "@/services/media";
import { saveProjectToDisk } from "@/services/project";
import type { MediaSourceMetadata } from "@/types/media";
import type { Project } from "../types/project";

interface ProjectState {
  activeProject: Project | null;
  projectPath: string | null;
  isDirty: boolean;
  revision: number;
  saveState: "idle" | "saving" | "saved" | "error";
  lastSaveError: string | null;
  missingMediaIds: string[];

  setProject: (project: Project, path: string | null) => void;
  updateProject: (updater: (draft: Project) => void) => void;
  saveProject: () => Promise<void>;
  saveProjectAs: (path: string) => Promise<void>;
  setSaveState: (state: "idle" | "saving" | "saved" | "error") => void;
  checkMediaStatus: () => Promise<void>;
  relinkMedia: (mediaId: string, newPath: string, newMetadata: MediaSourceMetadata) => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  activeProject: null,
  projectPath: null,
  isDirty: false,
  revision: 0,
  saveState: "idle",
  lastSaveError: null,
  missingMediaIds: [],

  setProject: (project, path) => {
    set((state) => ({
      activeProject: project,
      projectPath: path,
      isDirty: false,
      revision: state.revision + 1,
      saveState: "idle",
      lastSaveError: null,
      missingMediaIds: [],
    }));
    void get().checkMediaStatus();
  },

  updateProject: (updater) => {
    set((state) => {
      if (!state.activeProject) return state;

      const newProject = { ...state.activeProject };
      newProject.media = [...newProject.media];
      newProject.settings = { ...newProject.settings };
      updater(newProject);

      return {
        activeProject: newProject,
        isDirty: true,
        revision: state.revision + 1,
        saveState: "idle",
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

  checkMediaStatus: async () => {
    const { activeProject, revision } = get();
    if (!activeProject) return;

    const missingIds: string[] = [];
    for (const media of activeProject.media) {
      try {
        if (!(await checkMediaExists(media.path))) {
          missingIds.push(media.id);
        }
      } catch (e) {
        console.error(`Failed to check media path ${media.path}:`, e);
        missingIds.push(media.id);
      }
    }

    if (get().revision === revision) {
      set({ missingMediaIds: missingIds });
    }
  },

  relinkMedia: (mediaId, newPath, newMetadata) => {
    get().updateProject((draft) => {
      const mediaIndex = draft.media.findIndex((media) => media.id === mediaId);
      if (mediaIndex !== -1) {
        draft.media[mediaIndex] = {
          ...draft.media[mediaIndex],
          path: newPath,
          metadata: newMetadata,
        };
      }
    });

    set((state) => ({
      missingMediaIds: state.missingMediaIds.filter((id) => id !== mediaId),
    }));
  },
}));
