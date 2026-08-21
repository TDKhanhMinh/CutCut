import { Clapperboard, Home, Settings, Layers, FilePlus, FolderOpen, Save } from "lucide-react";
import { useAppStore } from "../../store";
import { useProjectStore } from "../../stores/useProjectStore";
import { useNavigate, useLocation } from "react-router-dom";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect } from "react";
import { Button } from "../ui/button";
import { createProject, loadProjectFromDisk } from "@/services/project";
import { useI18n } from "@/i18n";

export function Sidebar() {
  const { t } = useI18n();
  const sidebarOpen = useAppStore((state) => state.sidebarOpen);
  const location = useLocation();
  const {
    activeProject,
    saveState,
    isDirty,
    setProject,
    saveProject,
    saveProjectAs,
    lastSaveError,
  } = useProjectStore();

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
      const p = await createProject();
      const path = await save({
        filters: [{ name: "CutCut Project", extensions: ["cutcut"] }],
        defaultPath: "Untitled.cutcut",
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
        filters: [{ name: "CutCut Project", extensions: ["cutcut"] }],
        multiple: false,
      });
      if (selected) {
        const path = Array.isArray(selected) ? selected[0] : selected;
        const res = await loadProjectFromDisk(path);
        setProject(res.project, res.path);
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleSaveAs = async () => {
    if (!activeProject) return;

    const path = await save({
      filters: [{ name: "CutCut Project", extensions: ["cutcut"] }],
      defaultPath: "Untitled.cutcut",
    });
    if (path) {
      await saveProjectAs(path);
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
          label={t("nav.home")}
          path="/"
          active={location.pathname === "/"}
        />
        <NavItem
          icon={<Layers className="h-5 w-5" />}
          label={t("nav.editor")}
          path="/editor"
          active={location.pathname === "/editor"}
        />

        <div className="my-2 border-t border-border"></div>

        <Button
          variant="ghost"
          onClick={handleNewProject}
          className="flex w-full items-center justify-center gap-3 p-3 text-muted-foreground lg:justify-start lg:px-4"
        >
          <FilePlus className="h-5 w-5" />
          <span className="hidden text-sm font-medium lg:inline-block">{t("nav.newProject")}</span>
        </Button>

        <Button
          variant="ghost"
          onClick={handleOpenProject}
          className="flex w-full items-center justify-center gap-3 p-3 text-muted-foreground lg:justify-start lg:px-4"
        >
          <FolderOpen className="h-5 w-5" />
          <span className="hidden text-sm font-medium lg:inline-block">{t("nav.openProject")}</span>
        </Button>

        {activeProject && (
          <>
            <Button
              variant="ghost"
              onClick={() => saveProject()}
              className="flex w-full items-center justify-center gap-3 p-3 text-muted-foreground lg:justify-start lg:px-4"
            >
              <Save className="h-5 w-5" />
              <span className="hidden text-sm font-medium lg:inline-block">{t("nav.save")}</span>
            </Button>
            <Button
              variant="ghost"
              onClick={handleSaveAs}
              className="flex w-full items-center justify-center gap-3 p-3 text-muted-foreground lg:justify-start lg:px-4"
            >
              <Save className="h-5 w-5" />
              <span className="hidden text-sm font-medium lg:inline-block">{t("nav.saveAs")}</span>
            </Button>
          </>
        )}
      </nav>

      {activeProject && (
        <div className="w-full p-2 text-center text-xs font-medium text-muted-foreground lg:p-4 lg:text-left">
          {saveState === "saving" && <span className="text-yellow-500">{t("nav.saving")}</span>}
          {saveState === "saved" && <span className="text-green-500">{t("nav.saved")}</span>}
          {saveState === "error" && (
            <span className="text-red-500" title={lastSaveError ?? undefined}>
              {t("nav.saveFailed")}
            </span>
          )}
          {saveState === "idle" && isDirty && (
            <span className="text-yellow-500">{t("nav.unsaved")}</span>
          )}
        </div>
      )}

      <div className="w-full border-t border-border p-2 lg:p-4">
        <NavItem
          icon={<Settings className="h-5 w-5" />}
          label={t("common.settings")}
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
    <Button
      variant="ghost"
      onClick={handleClick}
      className={`flex w-full items-center justify-center gap-3 p-3 lg:justify-start lg:px-4 ${active ? "bg-primary/10 text-primary hover:bg-primary/20" : "text-muted-foreground"}`}
    >
      {icon}
      <span className="hidden text-sm font-medium lg:inline-block">{label}</span>
    </Button>
  );
}
