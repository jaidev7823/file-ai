import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";

export default function FileScanner() {
  const [files, setFiles] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [indexing, setIndexing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleScan = async () => {
    setLoading(true);
    setError(null);
    setFiles([]);

    try {
      const result = await invoke<string[]>("scan_text_files", {
        path: "C://Users/Jai Mishra/OneDrive/Documents",
      });
      setFiles(result);
    } catch (err: any) {
      setError(err?.toString() || "Scan failed");
    } finally {
      setLoading(false);
    }
  };

  const handleIndex = async () => {
    setIndexing(true);
    setError(null);
    try {
      await invoke<number>("scan_and_store_files", {
        path: "C://Users/Jai Mishra/OneDrive/Documents",
      });
    } catch (err: any) {
      setError(err?.toString() || "Indexing failed");
    } finally {
      setIndexing(false);
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
          {indexing ? "Indexing..." : "Index Now"}
        </Button>
      </div>

      {/* Error */}
      {error && <div className="text-sm text-red-500">{error}</div>}

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
