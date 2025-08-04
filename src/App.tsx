// we are here importing all the important thing which require for our project like useState invoke from tauri ui componenent and our own component like scanbutton and setting
// useState is react hook who let you update the state 
import { useState } from "react";
//  this to use tauri command which we created in src-tauri command.rs and invoke in lib.rs
import { invoke } from "@tauri-apps/api/core";
// shadcn ui componenet
import { Button } from "@/components/ui/button";
import ScanButton from "@/components/ScanButton";
import Settings from "@/pages/Settings";
// icons from lucide which we are importing this is used by shadcn
import { Settings as SettingsIcon, Home } from "lucide-react";

// This is the main function of our app(file-ai) when user open the app this function which got executed first
function App() {
  // this is the react state here we are changing the vlaue of currentPage with the help of setCurrentPage and userState<''> this part says it will be only this two pages and ('') says default page will be home
  const [currentPage, setCurrentPage] = useState<'home' | 'settings'>('home');
  // this is for debug a button it was here when i need to create shortcut for pop up search it;s simple to understand
  const testSearchWindow = async () => {
    try {
      // invoke is what we are using from tauri and toggle_search_window is rust function you can easily find this in our backend lib.rs
      await invoke("toggle_search_window");
      console.log("Search window toggle called successfully");
    } catch (error) {
      console.error("Failed to toggle search window:", error);
    }
  };

  // this is for changing the state of currentPage here we are checking if we are on setting page add a button name Back to Home 
  if (currentPage === 'settings') {
    // in react we return html element in this saying if setting page use this
    return (
      <div className="min-h-screen bg-white">
        <div className="border-b border-gray-200 p-4">
          <Button 
            // here we are changing the state of page with setCurrentPage when user click 
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

  // here we are returning one more html element
  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-10 bg-white text-black">
      <div className="absolute top-4 right-4">
        // similar like back to home page
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

      // here we are calling the main part of our page scanButton you can find this button in componenent/scanbutton.tsx
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
