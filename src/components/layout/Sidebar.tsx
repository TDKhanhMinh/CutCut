import { Clapperboard, Home, Settings, Layers } from "lucide-react";
import { useAppStore } from "../../store";
import { useNavigate, useLocation } from "react-router-dom";

export function Sidebar() {
  const sidebarOpen = useAppStore((state) => state.sidebarOpen);
  const location = useLocation();

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
      </nav>
      <div className="w-full p-2 lg:p-4">
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
