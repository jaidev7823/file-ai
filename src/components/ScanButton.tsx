import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Progress } from "@/components/ui/progress";

interface ScanProgress {
  current: number;
  total: number;
  current_file: string;
  stage: string;
}

interface ScanButtonProps {
  scanPaths?: string[];
  ignoredFolders?: string[];
}

export default function FileScanner({ scanPaths = [], ignoredFolders = [] }: ScanButtonProps) {
  const [files, setFiles] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [indexing, setIndexing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ScanProgress | null>(null);

  const handleScan = async () => {
    setLoading(true);
    setError(null);
    setFiles([]);

    try {
      // Use the first scan path or fallback to default
      const pathToScan = scanPaths.length > 0 ? scanPaths[0] : "C://Users/Jai Mishra/OneDrive/Documents";
      
      const result = await invoke<string[]>("scan_text_files", {
        path: pathToScan,
        ignoredFolders: ignoredFolders,
      });
      setFiles(result);
    } catch (err: any) {
      setError(err?.toString() || "Scan failed");
    } finally {
      setLoading(false);
    }
  };

  // Set up progress listener
  useEffect(() => {
    const setupProgressListener = async () => {
      const unlisten = await listen<ScanProgress>("scan_progress", (event) => {
        setProgress(event.payload);

        // Reset progress when complete
        if (event.payload.stage === "complete") {
          setTimeout(() => {
            setProgress(null);
            setIndexing(false);
          }, 2000); // Show completion for 2 seconds
        }
      });

      return unlisten;
    };

    let unlisten: (() => void) | null = null;
    setupProgressListener().then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const handleIndex = async () => {
    setIndexing(true);
    setError(null);
    setProgress(null);

    try {
      // Use the first scan path or fallback to default
      const pathToScan = scanPaths.length > 0 ? scanPaths[0] : "C://Users/Jai Mishra/OneDrive/Documents";
      
      await invoke<number>("scan_and_store_files_with_progress", {
        path: pathToScan,
        ignoredFolders: ignoredFolders,
      });
    } catch (err: any) {
      setError(err?.toString() || "Indexing failed");
      setIndexing(false);
      setProgress(null);
    }
  };

  return (
    <div className="space-y-6">
      {/* Top Buttons */}
      <div className="flex justify-between items-center gap-4">
        <Button onClick={handleScan} disabled={loading}>
          {loading ? "Scanning..." : "Scan Text Files"}
        </Button>
        <Button onClick={handleIndex} variant="secondary" disabled={indexing}>
          {indexing ? (progress ? `${progress.stage}...` : "Indexing...") : "Index Now"}
        </Button>
      </div>

      {/* Error */}
      {error && <div className="text-sm text-red-500">{error}</div>}

      {/* Progress Bar */}
      {progress && (
        <Card className="p-4 w-full max-w-2xl mx-auto">
          <div className="space-y-3">
            <div className="flex justify-between items-center">
              <span className="text-sm font-medium capitalize">
                {progress.stage === "scanning" && "🔍 Scanning Files"}
                {progress.stage === "reading" && "📖 Reading Files"}
                {progress.stage === "embedding" && "🧠 Generating Embeddings"}
                {progress.stage === "storing" && "💾 Storing in Database"}
                {progress.stage === "complete" && "✅ Complete"}
              </span>
              <span className="text-sm text-muted-foreground whitespace-nowrap">
                {progress.current} / {progress.total} ({progress.total > 0 ? Math.round((progress.current / progress.total) * 100) : 0}%)
              </span>
            </div>

            <Progress
              value={progress.total > 0 ? (progress.current / progress.total) * 100 : 0}
              className="w-full"
            />

            <div className="text-xs text-muted-foreground truncate max-w-full">
              <span className="font-mono">
                {progress.current_file.length > 80 
                  ? `...${progress.current_file.slice(-77)}` 
                  : progress.current_file}
              </span>
            </div>
          </div>
        </Card>
      )}

      {/* File List */}
      <ScrollArea className="h-[30rem] w-full">
        <div className="space-y-2 w-full max-w-2xl mx-auto">
          {files.map((filePath, idx) => {
            const fileName = filePath.split("\\").pop() || filePath;

            return (
              <Card
                key={idx}
                className="p-2 shadow-sm w-full max-w-full overflow-hidden"
              >
                <div className="flex flex-col space-y-1 px-2 py-1">
                  <h3 className="text-sm font-medium leading-tight truncate">
                    {idx + 1}. {fileName}
                  </h3>
                  <p className="text-xs text-muted-foreground font-mono truncate">
                    {filePath}
                  </p>
                </div>
              </Card>

            );
          })}
        </div>
      </ScrollArea>
    </div>
  );
}
