import { useState } from "react";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";

export default function Home() {
  const [isScanning, setIsScanning] = useState(false);
  const [scanResults, setScanResults] = useState<string[]>([]);

  const handleScanFiles = async () => {
    setIsScanning(true);
    try {
      // You can modify this path or make it configurable
      const results = await invoke<string[]>("scan_text_files", { 
        path: "C:\\" // Default to C: drive, you can change this
      });
      setScanResults(results);
      console.log("Scan results:", results);
    } catch (error) {
      console.error("Error scanning files:", error);
    } finally {
      setIsScanning(false);
    }
  };

  return (
    <div className="flex flex-col items-center justify-center min-h-full space-y-6">
      <div className="text-center space-y-4">
        <h1 className="text-4xl font-bold text-foreground">
          Welcome to AI OS 👋
        </h1>
        <p className="text-lg text-muted-foreground max-w-md">
          Scan and analyze your files with AI-powered insights
        </p>
      </div>

      <Button 
        onClick={handleScanFiles}
        disabled={isScanning}
        size="lg"
        className="px-8 py-3 text-lg"
      >
        {isScanning ? "Scanning..." : "Scan Your Files"}
      </Button>

      {scanResults.length > 0 && (
        <div className="mt-8 w-full max-w-2xl">
          <h2 className="text-xl font-semibold mb-4">
            Found {scanResults.length} files
          </h2>
          <div className="bg-card border rounded-lg p-4 max-h-60 overflow-y-auto">
            {scanResults.slice(0, 10).map((file, index) => (
              <div key={index} className="text-sm text-muted-foreground py-1">
                {file}
              </div>
            ))}
            {scanResults.length > 10 && (
              <div className="text-sm text-muted-foreground py-1 font-medium">
                ... and {scanResults.length - 10} more files
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}