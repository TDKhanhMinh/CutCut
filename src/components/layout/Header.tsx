import { Bell, Menu } from "lucide-react";
import { useAppStore } from "../../store";

export function Header() {
  const toggleSidebar = useAppStore((state) => state.toggleSidebar);

  return (
    <header className="flex h-14 shrink-0 items-center justify-between border-b border-border bg-background px-4">
      <div className="flex items-center gap-4">
        <button
          onClick={toggleSidebar}
          className="rounded-md p-2 text-muted-foreground transition-colors hover:bg-muted lg:hidden"
        >
          <Menu className="h-5 w-5" />
        </button>
        <h2 className="max-w-[200px] truncate text-sm font-semibold sm:max-w-xs">
          Untitled Project
        </h2>
      </div>
      <div className="flex items-center gap-2">
        <button className="rounded-md p-2 text-muted-foreground transition-colors hover:bg-muted">
          <Bell className="h-5 w-5" />
        </button>
        <div className="ml-2 h-8 w-8 rounded-full border border-primary/30 bg-primary/20"></div>
      </div>
    </header>
  );
}
