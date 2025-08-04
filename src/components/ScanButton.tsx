// task of this file
// Scans and lists text files from your system
// Shows progress while indexing files (with stages like scanning, reading, embedding, storing)
// Handles errors and updates UI
// Listens to backend events in real-time using useEffect

// here we are using useState as explain in app.tsx but we are here using useEffect
// we use useEffect hook when we want to run code when component load or when value change we use useState to change the value we use useEffect to run something after value change
import { useState, useEffect } from "react";
// tauri command import
import { invoke } from "@tauri-apps/api/core";
// listen let frontend to know event from backend like if we are running a loop in function in each loop we can send a value to listen and frontend will know how many loops has completed
import { listen } from "@tauri-apps/api/event";
// shadcn components
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Progress } from "@/components/ui/progress";

// well this where we are giving type to returned value from backend this is what we are going to get from backend when listning to progress
interface ScanProgress {
  current: number;
  total: number;
  current_file: string;
  stage: string;
}

// this is also type for letting know component which path to crawl which to ignore optional part of page
interface ScanButtonProps {
  scanPaths?: string[];
  ignoredFolders?: string[];
}

// we are using export which mean if any page claling this component run this function which fileScanner
// who ask for two argument scanPaths, ignoredFOlders and we have scanButtonProps this is optional and have remove after adding new feat:
export default function FileScanner({ scanPaths = [], ignoredFolders = [] }: ScanButtonProps) {
  // declaring diffrent type of state use for managing progress bar file scan 
  // store list of scanned file path
  const [files, setFiles] = useState<string[]>([]);
  // whethere scan is running
  const [loading, setLoading] = useState(false);
  // where indexing is in progress
  const [indexing, setIndexing] = useState(false);
  // store errors messsage
  const [error, setError] = useState<string | null>(null);
  // Hold progress data send by tauri backend
  const [progress, setProgress] = useState<ScanProgress | null>(null);

  // here is the function run when do user click scan file
  const handleScan = async () => {
    // sets loadng to true saying started
    setLoading(true);
    // set error to null to clear previos errors
    setError(null);
    // and add blank list in files state
    setFiles([]);

    try {
      // This is to find out is scanPaths have value if not use default one
      const pathToScan = scanPaths.length > 0 ? scanPaths[0] : "C://Users/Jai Mishra/OneDrive/Documents";
      
      // here we are invoking a command from backend scan_text_files sending argumant pathToScan and ignoredFolders 
      const result = await invoke<string[]>("scan_text_files", {
        path: pathToScan,
        ignoredFolders: ignoredFolders,
      });
      // and setting returned file in files
      setFiles(result);
    } catch (err: any) {
      setError(err?.toString() || "Scan failed");
    } finally {
      setLoading(false);
    }
  };

  // Set up progress listener we are using useEffect to check how much file is indexed
  useEffect(() => {
    const setupProgressListener = async () => {
      // here we are using tauri listen function and ScanProgress types and colling scan_progress from backend it will return value until job is done
      const unlisten = await listen<ScanProgress>("scan_progress", (event) => {
        // here event is object sent from tauri backend event.payload is the data sent in the event
        // this will return same data like scanprogress ts
        setProgress(event.payload);

        // Reset progress when complete when stage become complete it will restart everything may be for next round like embedding or something
        if (event.payload.stage === "complete") {
          setTimeout(() => {
            setProgress(null);
            setIndexing(false);
          }, 2000); // Show completion for 2 seconds
        }
      });

      return unlisten;
    };

    // unlisten is our own function created for unsubscribe the emit 
    let unlisten: (() => void) | null = null;
    setupProgressListener().then((fn) => {
      unlisten = fn;
    });

    // it check if unlisten true then run unlisten function
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  // here we are handling indexing of file main embedding and other
  const handleIndex = async () => {
    // first resetting states
    setIndexing(true);
    setError(null);
    setProgress(null);

    try {
      // it check does scapath have any path if not use default one simple if else with ? : 
      const pathToScan = scanPaths.length > 0 ? scanPaths[0] : "C://Users/Jai Mishra/OneDrive/Documents";
      
      // invoking the rust command scan_and_store_files_with_progress
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
          {/* check if loading then scanning... or scan text files simple */}
          {loading ? "Scanning..." : "Scan Text Files"}
        </Button>
        <Button onClick={handleIndex} variant="secondary" disabled={indexing}>
          {/* similar logic */}
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
