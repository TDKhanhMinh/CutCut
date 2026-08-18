import { Bell, Menu, Sun, Moon } from "lucide-react";
import { useAppStore } from "../../store";
import { Button } from "../ui/button";
import { useTheme } from "../theme-provider";

export function Header() {
  const toggleSidebar = useAppStore((state) => state.toggleSidebar);
  const { theme, setTheme } = useTheme();

  return (
    <header className="flex h-14 shrink-0 items-center justify-between border-b border-border bg-background px-4">
      <div className="flex items-center gap-4">
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleSidebar}
          className="lg:hidden text-muted-foreground"
        >
          <Menu className="h-5 w-5" />
        </Button>
        <h2 className="max-w-[200px] truncate text-sm font-semibold sm:max-w-xs">
          Untitled Project
        </h2>
      </div>
      <div className="flex items-center gap-2">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
          className="text-muted-foreground relative"
        >
          <Sun className="h-5 w-5 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
          <Moon className="absolute h-5 w-5 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
          <span className="sr-only">Toggle theme</span>
        </Button>
        <Button variant="ghost" size="icon" className="text-muted-foreground">
          <Bell className="h-5 w-5" />
        </Button>
        <div className="ml-2 h-8 w-8 rounded-full border border-primary/30 bg-primary/20"></div>
      </div>
    </header>
  );
}
