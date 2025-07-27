import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import ScanButton from "@/components/ScanButton";

function App() {

  const testSearchWindow = async () => {
    try {
      await invoke("toggle_search_window");
      console.log("Search window toggle called successfully");
    } catch (error) {
      console.error("Failed to toggle search window:", error);
    }
  };

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-10 bg-white text-black">
      <div className="text-center mb-8">
        <h1 className="text-4xl font-bold mb-4">
          Welcome to AI OS 👋
        </h1>
        <p className="text-xl text-gray-600 max-w-md">
          Scan and analyze your files with AI-powered insights
        </p>
        <p className="text-sm text-gray-500 mt-2">
          Press <kbd className="px-2 py-1 bg-gray-100 rounded text-xs">Ctrl+Shift+P</kbd> to open global search
        </p>
        <p className="text-xs text-gray-400 mt-1">
          The search will appear as a system-wide overlay, even when this app is minimized
        </p>
      </div>

      <div className="space-y-4">
        <ScanButton />
        
        {/* Debug button to test search window */}
        <Button onClick={testSearchWindow} variant="outline">
          🔍 Test Search Window (Debug)
        </Button>
      </div>
    </div>
  );
}

export default App;
