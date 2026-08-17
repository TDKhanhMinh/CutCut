import { create } from "zustand";

interface AppShellState {
  sidebarOpen: boolean;
  setSidebarOpen: (open: boolean) => void;
  toggleSidebar: () => void;
  currentProject: string | null;
  setCurrentProject: (id: string | null) => void;
}

export const useAppStore = create<AppShellState>((set) => ({
  sidebarOpen: false,
  setSidebarOpen: (open) => set({ sidebarOpen: open }),
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  currentProject: null,
  setCurrentProject: (id) => set({ currentProject: id }),
}));
