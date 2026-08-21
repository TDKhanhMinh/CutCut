import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { ThemeProvider } from "./components/theme-provider";
import { TooltipProvider } from "./components/ui/tooltip";
import { AppErrorBoundary } from "./components/AppErrorBoundary";
import { I18nProvider } from "./i18n";
import { telemetry } from "./services/telemetry";

telemetry.track("startup");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AppErrorBoundary>
      <ThemeProvider defaultTheme="dark" storageKey="vite-ui-theme">
        <I18nProvider>
          <TooltipProvider>
            <App />
          </TooltipProvider>
        </I18nProvider>
      </ThemeProvider>
    </AppErrorBoundary>
  </React.StrictMode>,
);
