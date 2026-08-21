import { Component, type ErrorInfo, type ReactNode } from "react";
import { Button } from "@/components/ui/button";

interface AppErrorBoundaryProps {
  children: ReactNode;
}

interface AppErrorBoundaryState {
  hasError: boolean;
}

export class AppErrorBoundary extends Component<AppErrorBoundaryProps, AppErrorBoundaryState> {
  state: AppErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(): AppErrorBoundaryState {
    return { hasError: true };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Unhandled application error", error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <main className="grid min-h-screen place-items-center bg-background p-6 text-foreground">
          <div className="max-w-md space-y-3 text-center">
            <h1 className="text-xl font-semibold">CutCut gặp lỗi ngoài dự kiến</h1>
            <p className="text-sm text-muted-foreground">
              Phiên làm việc hiện tại vẫn an toàn. Hãy thử tải lại giao diện.
            </p>
            <Button onClick={() => window.location.reload()}>Tải lại</Button>
          </div>
        </main>
      );
    }

    return this.props.children;
  }
}
