export function Workspace() {
  return (
    <div className="flex h-full w-full flex-1 items-center justify-center bg-muted/10 p-4">
      <div className="max-w-md space-y-4 rounded-xl border border-dashed border-border bg-background p-8 text-center shadow-sm">
        <h3 className="text-lg font-medium">Workspace Area</h3>
        <p className="text-sm text-muted-foreground">
          This is the main editor canvas where video preview, transcript, and timeline will be
          placed.
        </p>
      </div>
    </div>
  );
}
