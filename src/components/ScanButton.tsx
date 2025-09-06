import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";

interface ScanProgress {
  current: number;
  total: number;
  current_file: string;
  stage: string;
}

interface ScannedFile {
  path: string;
  content_processed: boolean;
}

interface ScanButtonProps {}

export default function ({}: ScanButtonProps) {
  const [files, setFiles] = useState<ScannedFile[]>([]);
  const [loadingScan, setLoadingScan] = useState(false);
  const [loadingIndex, setLoadingIndex] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ScanProgress | null>(null);

  const handleScan = async () => {
    setLoadingScan(true);
    setError(null);
    setFiles([]); // Clear previous files when starting a new scan
    setProgress(null); // Clear progress when starting a new scan

    try {
      const result = await invoke<ScannedFile[]>("scan_text_files");
      setFiles(result || []);
    } catch (err: any) {
      setError(err?.toString() || "Scan failed");
    } finally {
      setLoadingScan(false);
    }
  };

  // Update the useEffect for progress listening
  useEffect(() => {
    const setup = async () => {
      try {
        const unlisten = await listen<ScanProgress>(
          "scan_progress",
          (event) => {
            const payload = event.payload;
            console.log("Progress event received:", payload);

            // Only update progress if we have valid data
            if (payload && typeof payload.current !== "undefined") {
              setProgress(payload);

              // Handle completion
              if (payload.stage === "complete") {
                setLoadingIndex(false);
                setLoadingScan(false);
                console.log("Scan/Index completed");
              }
            }
          }
        );

        console.log("Progress listener set up successfully");
        return unlisten;
      } catch (e) {
        console.error("Failed to set up progress listener:", e);
        setError("Failed to monitor progress");
        return () => {};
      }
    };

    setup();
    return () => {};
  }, []);

  const handleIndex = async () => {
    setLoadingIndex(true);
    setError(null);
    setProgress({ current: 0, total: 0, current_file: "", stage: "scanning" });

    try {
      await invoke<number>("run_full_scan_and_index");
    } catch (err: any) {
      setError(err?.toString() || "Indexing failed");
      setLoadingIndex(false); // Consider adding this to reset the loading state on error
    }
  };
  const handleDriveIndex = async () => {
    setLoadingIndex(true);
    setError(null);
    setProgress({ current: 0, total: 0, current_file: "", stage: "scanning" });

    try {
      await invoke<number>("scan_drives_metadata");
    } catch (err: any) {
      setError(err?.toString() || "Drive indexing failed");
      setLoadingIndex(false);
      setProgress(null);
    }
  };
  return (
    <div className="space-y-6">
      {/* Top Buttons */}
      <div className="flex justify-between items-center gap-4">
        <Button onClick={handleScan} disabled={loadingScan || loadingIndex}>
          {loadingScan ? "Scanning..." : "Scan Now"}
        </Button>

        <Button
          onClick={handleIndex}
          variant="secondary"
          disabled={loadingIndex || loadingScan}
        >
          {loadingIndex
            ? progress
              ? `${progress.stage}...`
              : "Scanning & Indexing..."
            : "Scan and Index Now"}
        </Button>

        <Button
          onClick={handleDriveIndex}
          variant="destructive"
          disabled={loadingIndex || loadingScan}
        >
          {loadingIndex
            ? progress
              ? `${progress.stage}...`
              : "Indexing Drive..."
            : "Index Whole Drive"}
        </Button>
      </div>

      {/* Error */}
      {error && <div className="text-sm text-red-500">{error}</div>}

      {/* Progress Bar */}
      {progress && (
        <Card className="p-4 w-[600px] mx-auto">
          <div className="space-y-3 w-full">
            <div className="flex justify-between items-center">
              <span className="text-sm font-medium capitalize">
                {(() => {
                  switch (progress.stage) {
                    case "scanning":
                      return "🔍 Scanning Files";
                    case "reading metadata":
                      return "📖 Reading Metadata";
                    case "reading":
                      return "📖 Reading Files";
                    case "embedding":
                      return "🧠 Generating Embeddings";
                    case "storing":
                      return "💾 Storing in Database";
                    case "phase2_discovery":
                      return "🔍 Discovering Drives";
                    case "phase2_scanning":
                      return "📱 Scanning System";
                    case "phase2_scan_complete":
                      return "✅ Drive Scan Complete";
                    case "complete":
                      return "✅ Complete";
                    default:
                      return progress.stage;
                  }
                })()}
              </span>
              {progress.total > 0 && (
                <span className="text-sm text-muted-foreground whitespace-nowrap">
                  {progress.current.toLocaleString()} /{" "}
                  {progress.total.toLocaleString()} (
                  {Math.round((progress.current / progress.total) * 100)}%)
                </span>
              )}
            </div>

            <div className="w-full max-w-full">
              <Progress
                value={
                  progress.total > 0
                    ? (progress.current / progress.total) * 100
                    : 0
                }
                className="w-full h-2 rounded-full"
              />
            </div>

            {progress.current_file && (
              <div className="w-full">
                <p className="text-xs text-muted-foreground font-mono truncate">
                  {progress.current_file}
                </p>
              </div>
            )}
          </div>
        </Card>
      )}

      {/* File List */}
      <ScrollArea className="h-[30rem] w-full">
        <div className="space-y-2 w-full max-w-2xl mx-auto">
          {files.map((file, idx) => {
            const fileName = file.path.split(/[\/]/).pop() || file.path;

            return (
              <Card
                key={idx}
                className="p-2 shadow-sm w-full max-w-full overflow-hidden"
              >
                <div className="flex flex-col space-y-1 px-2 py-1">
                  <div className="flex justify-between items-center">
                    <h3 className="text-sm font-medium leading-tight truncate">
                      {idx + 1}. {fileName}
                    </h3>
                    {!file.content_processed && (
                      <Badge variant="outline">Metadata Only</Badge>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground font-mono truncate max-w-full">
                    {file.path}
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
