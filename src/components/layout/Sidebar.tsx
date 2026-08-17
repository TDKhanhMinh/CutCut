import { Clapperboard, Home, Settings, Layers, FilePlus, FolderOpen, Save } from "lucide-react";
import { useAppStore } from "../../store";
import { useProjectStore } from "../../stores/useProjectStore";
import { useNavigate, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Project } from "../../types/project";
import { useEffect } from "react";

export function Sidebar() {
  const sidebarOpen = useAppStore((state) => state.sidebarOpen);
  const location = useLocation();
  const { activeProject, saveState, isDirty, setProject, saveProject } = useProjectStore();

  // Autosave logic (debounce 3s)
  useEffect(() => {
    if (!isDirty || !activeProject) return;
    const timer = setTimeout(() => {
      saveProject();
    }, 3000);
    return () => clearTimeout(timer);
  }, [isDirty, activeProject, saveProject]);

  const handleNewProject = async () => {
    try {
      const p = await invoke<Project>('create_project');
      const path = await save({
        filters: [{ name: 'CutCut Project', extensions: ['cutcut'] }],
        defaultPath: 'Untitled.cutcut'
      });
      if (path) {
        setProject(p, path);
        // Force an initial save
        useProjectStore.setState({ isDirty: true });
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleOpenProject = async () => {
    try {
      const selected = await open({
        filters: [{ name: 'CutCut Project', extensions: ['cutcut'] }],
        multiple: false
      });
      if (selected) {
        const path = Array.isArray(selected) ? selected[0] : selected;
        const res = await invoke<{project: Project, path: string}>('load_project_from_disk', { path });
        setProject(res.project, res.path);
      }
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <aside
      className={`fixed inset-y-0 left-0 z-50 transform ${sidebarOpen ? "translate-x-0" : "-translate-x-full"} flex w-64 shrink-0 flex-col items-center border-r border-border bg-background transition-transform duration-300 sm:w-20 lg:relative lg:w-64 lg:translate-x-0 lg:items-start lg:bg-muted/20`}
    >
      <div className="flex h-14 w-full items-center justify-center border-b border-border lg:px-6">
        <Clapperboard className="h-6 w-6 text-primary" />
        <span className="ml-3 hidden text-lg font-semibold lg:inline-block">CutCut</span>
      </div>
      <nav className="flex w-full flex-1 flex-col gap-2 p-2 lg:p-4">
        <NavItem
          icon={<Home className="h-5 w-5" />}
          label="Home"
          path="/"
          active={location.pathname === "/"}
        />
        <NavItem
          icon={<Layers className="h-5 w-5" />}
          label="Editor"
          path="/editor"
          active={location.pathname === "/editor"}
        />

        <div className="my-2 border-t border-border"></div>
        
        <button
          onClick={handleNewProject}
          className="flex w-full items-center justify-center gap-3 rounded-md p-3 transition-colors hover:bg-muted lg:justify-start lg:px-4 text-muted-foreground"
        >
          <FilePlus className="h-5 w-5" />
          <span className="hidden text-sm font-medium lg:inline-block">New Project</span>
        </button>

        <button
          onClick={handleOpenProject}
          className="flex w-full items-center justify-center gap-3 rounded-md p-3 transition-colors hover:bg-muted lg:justify-start lg:px-4 text-muted-foreground"
        >
          <FolderOpen className="h-5 w-5" />
          <span className="hidden text-sm font-medium lg:inline-block">Open Project</span>
        </button>
        
        {activeProject && (
          <button
            onClick={() => saveProject()}
            className="flex w-full items-center justify-center gap-3 rounded-md p-3 transition-colors hover:bg-muted lg:justify-start lg:px-4 text-muted-foreground"
          >
            <Save className="h-5 w-5" />
            <span className="hidden text-sm font-medium lg:inline-block">Save</span>
          </button>
        )}
      </nav>

      {activeProject && (
          <div className="w-full p-2 lg:p-4 text-xs font-medium text-center lg:text-left text-muted-foreground">
              {saveState === 'saving' && <span className="text-yellow-500">Saving...</span>}
              {saveState === 'saved' && <span className="text-green-500">All changes saved</span>}
              {saveState === 'error' && <span className="text-red-500">Save failed!</span>}
              {saveState === 'idle' && isDirty && <span className="text-yellow-500">Unsaved changes</span>}
          </div>
      )}

      <div className="w-full p-2 lg:p-4 border-t border-border">
        <NavItem
          icon={<Settings className="h-5 w-5" />}
          label="Settings"
          path="/settings"
          active={location.pathname === "/settings"}
        />
      </div>
    </aside>
  );
}

function NavItem({
  icon,
  label,
  path,
  active,
}: {
  icon: React.ReactNode;
  label: string;
  path: string;
  active?: boolean;
}) {
  const navigate = useNavigate();
  const setSidebarOpen = useAppStore((state) => state.setSidebarOpen);

  const handleClick = () => {
    navigate(path);
    setSidebarOpen(false);
  };

  return (
    <button
      onClick={handleClick}
      className={`flex w-full items-center justify-center gap-3 rounded-md p-3 transition-colors hover:bg-muted lg:justify-start lg:px-4 ${active ? "bg-primary/10 text-primary" : "text-muted-foreground"}`}
    >
      {icon}
      <span className="hidden text-sm font-medium lg:inline-block">{label}</span>
    </button>
  );
}
