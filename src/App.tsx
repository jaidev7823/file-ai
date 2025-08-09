import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import ScanButton from "@/components/ScanButton";
import Settings from "@/pages/Settings";
import { Settings as SettingsIcon, Home } from "lucide-react";

function App() {
  const [currentPage, setCurrentPage] = useState<'home' | 'settings'>('home');

  const testSearchWindow = async () => {
    try {
      await invoke("toggle_search_window");
      console.log("Search window toggle called successfully");
    } catch (error) {
      console.error("Failed to toggle search window:", error);
    }
  };

  if (currentPage === 'settings') {
    return (
      <div className="min-h-screen bg-white">
        <div className="border-b border-gray-200 p-4">
          <Button 
            onClick={() => setCurrentPage('home')} 
            variant="ghost" 
            className="flex items-center gap-2"
          >
            <Home className="h-4 w-4" />
            Back to Home
          </Button>
        </div>
        <Settings />
      </div>
    );
  }

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-10 bg-white text-black">
      <div className="absolute top-4 right-4">
        <Button 
          onClick={() => setCurrentPage('settings')} 
          variant="outline" 
          size="sm"
          className="flex items-center gap-2"
        >
          <SettingsIcon className="h-4 w-4" />
          Settings
        </Button>
      </div>

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
        
      </div>
    </div>
  );
}

export default App;
