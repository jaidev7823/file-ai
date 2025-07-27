import { useState } from "react";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";

export default function ScanButton() {
  const [files, setFiles] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleScan = async () => {
    setLoading(true);
    setError(null);

    try {
      const result = await invoke<string[]>("scan_text_files", {
        path: "C://Users/Jai Mishra/OneDrive/Documents",
      });
      setFiles(result);
    } catch (err: any) {
      setError(err?.toString() || "Something went wrong");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-4">
      <Button onClick={handleScan} disabled={loading}>
        {loading ? "Scanning..." : "Scan for Text Files"}
      </Button>

      {error && <div className="text-red-500 text-sm">{error}</div>}

      <ul className="text-xs font-mono max-h-64 overflow-auto">
        {files.map((file, idx) => (
          <li key={idx}>{file}</li>
        ))}
      </ul>
    </div>
  );
}
