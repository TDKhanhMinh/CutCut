import { MediaImporter } from "../components/media/MediaImporter";

export function Home() {
  return (
    <div className="flex-1 p-8">
      <h1 className="mb-4 text-2xl font-bold">Home</h1>
      <p className="text-muted-foreground mb-8">Welcome to CutCut Editor.</p>
      <MediaImporter />
    </div>
  );
}
