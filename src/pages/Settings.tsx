import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { X, Plus, Settings as SettingsIcon, FileX } from "lucide-react";
import { Separator } from "@/components/ui/separator";
import ScanButton from "@/components/ScanButton";

interface ScanSettings {
  ignoredExtensions: string[];
  ignoredFolders: string[];
}

export default function Settings() {
  // State for storing scan settings (extensions and folders to ignore)
  const [scanSettings, setScanSettings] = useState<ScanSettings>({
    ignoredExtensions: [],
    ignoredFolders: []
  });

  // State for new extension/folder input
  const [newExtension, setNewExtension] = useState("");
  const [newIgnoreFolder, setNewIgnoreFolder] = useState("");
  const [loading, setLoading] = useState(false);
  
  // State for managing visibility of extensions when there are many
  const [showAllExtensions, setShowAllExtensions] = useState(false);
  const EXTENSION_LIMIT = 10; // Show only 10 extensions by default

  // Load settings from backend when component mounts
  useEffect(() => {
    const loadAllSettings = async () => {
      setLoading(true);
      try {
        // Fetch both extensions and folders in parallel
        const [extensions, folders] = await Promise.all([
          invoke<string[]>("get_included_extensions"),
          invoke<string[]>("get_excluded_paths")
        ]);

        setScanSettings({
          ignoredExtensions: extensions,
          ignoredFolders: folders,
        });
      } catch (error) {
        console.error("Failed to load settings from database:", error);
        // Fallback to some defaults if loading fails
        setScanSettings({
          ignoredExtensions: ["log", "tmp"],
          ignoredFolders: ["node_modules", ".git", "target"]
        });
      } finally {
        setLoading(false);
      }
    };

    loadAllSettings();
  }, []);

  // Add a new extension to the included list
  const addIncludedExtension = async () => {
    // Normalize by removing leading dot and trimming whitespace
    const extension = newExtension.trim().replace(/^\./, "");
    console.log("extension working");

    if (!extension) return;
    console.log("return working");

    if (!scanSettings.ignoredExtensions.includes(extension)) {
      console.log("ifworking");

      try {
      console.log("working");

        // Call backend to add the extension
        await invoke("add_included_extension", { extension });
        
        // Update UI state on success
        setScanSettings(prev => ({
          ...prev,
          ignoredExtensions: [...prev.ignoredExtensions, extension]
        }));
        setNewExtension("");
      } catch (error) {
        console.log("not working");
        console.error("Failed to add ignored extension:", error);
      }
    }
  };

  // Remove an extension from the included list
  const removeIgnoredExtension = async (extToRemove: string) => {
    try {
      // Call backend to remove the extension
      await invoke("remove_included_extension", { extension: extToRemove });
      
      // Update UI state
      setScanSettings(prev => ({
        ...prev,
        ignoredExtensions: prev.ignoredExtensions.filter(ext => ext !== extToRemove)
      }));
    } catch (error) {
      console.error("Failed to remove ignored extension:", error);
    }
  };

  // Add a new folder to the excluded paths
  const addIgnoreFolder = async () => {
    const folderName = newIgnoreFolder.trim();
    if (!folderName) return;

    if (!scanSettings.ignoredFolders.includes(folderName)) {
      try {
        // Call backend to add the folder
        await invoke("add_excluded_path", { path: folderName });
        
        // Update UI state
        setScanSettings(prev => ({
          ...prev,
          ignoredFolders: [...prev.ignoredFolders, folderName]
        }));
        setNewIgnoreFolder("");
      } catch (error) {
        console.error("Failed to add ignored folder:", error);
      }
    }
  };

  // Remove a folder from the excluded paths
  const removeIgnoreFolder = async (folderToRemove: string) => {
    try {
      // Call backend to remove the folder
      await invoke("remove_excluded_path", { path: folderToRemove });
      
      // Update UI state
      setScanSettings(prev => ({
        ...prev,
        ignoredFolders: prev.ignoredFolders.filter(folder => folder !== folderToRemove)
      }));
    } catch (error) {
      console.error("Failed to remove ignored folder:", error);
    }
  };

  // Determine which extensions to display based on showAllExtensions state
  const displayedExtensions = showAllExtensions 
    ? scanSettings.ignoredExtensions 
    : scanSettings.ignoredExtensions.slice(0, EXTENSION_LIMIT);

  return (
    <div className="p-6 max-w-4xl mx-auto space-y-6">
      <div className="flex items-center gap-2 mb-6">
        <SettingsIcon className="h-6 w-6" />
        <h1 className="text-2xl font-bold">File Indexing Settings</h1>
      </div>

      {/* Included Extensions Configuration Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FileX className="h-5 w-5" />
            Included Extensions
          </CardTitle>
          <CardDescription>
            Only files with these extensions will be scanned. Add extensions without the dot (e.g., "tsx", "md", "py").
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-2">
            {displayedExtensions.map((ext, index) => (
              <Badge key={index} variant="secondary" className="flex items-center gap-1 text-sm">
                .{ext}
                <button
                  onClick={() => removeIgnoredExtension(ext)}
                  className="ml-1 rounded-full hover:bg-gray-300 p-0.5"
                  aria-label={`Remove ${ext} extension`}
                >
                  <X className="h-3 w-3" />
                </button>
              </Badge>
            ))}
            
            {/* Show "Show More/Hide" button when there are more than EXTENSION_LIMIT extensions */}
            {scanSettings.ignoredExtensions.length > EXTENSION_LIMIT && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setShowAllExtensions(!showAllExtensions)}
                className="text-sm"
              >
                {showAllExtensions ? 'Hide' : `Show All (${scanSettings.ignoredExtensions.length})`}
              </Button>
            )}
          </div>

          <div className="flex gap-2">
            <Input
              placeholder="Type extension (e.g., tsx, md, py)"
              value={newExtension}
              onChange={(e) => setNewExtension(e.target.value)}
              onKeyPress={(e) => e.key === "Enter" && addIncludedExtension ()}
              className="flex-1"
            />
            <Button 
              onClick={addIncludedExtension} 
              disabled={!newExtension.trim()}
              aria-label="Add extension"
            >
              <Plus className="h-4 w-4 mr-1" /> Add Extension
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Ignored Folders Configuration */}
      <Card>
        <CardHeader>
          <CardTitle>Ignored Folders</CardTitle>
          <CardDescription>
            These folders will be completely skipped during scanning (e.g., "node_modules", "target").
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-2">
            {scanSettings.ignoredFolders.map((folder, index) => (
              <Badge key={index} variant="secondary" className="flex items-center gap-1 text-sm">
                {folder}
                <button
                  onClick={() => removeIgnoreFolder(folder)}
                  className="ml-1 rounded-full hover:bg-gray-300 p-0.5"
                  aria-label={`Remove ${folder} folder`}
                >
                  <X className="h-3 w-3" />
                </button>
              </Badge>
            ))}
          </div>

          <div className="flex gap-2">
            <Input
              placeholder="Type folder name (e.g., node_modules)"
              value={newIgnoreFolder}
              onChange={(e) => setNewIgnoreFolder(e.target.value)}
              onKeyPress={(e) => e.key === "Enter" && addIgnoreFolder()}
              className="flex-1"
            />
            <Button 
              onClick={addIgnoreFolder} 
              disabled={!newIgnoreFolder.trim()}
              aria-label="Add folder"
            >
              <Plus className="h-4 w-4 mr-1" /> Add Folder
            </Button>
          </div>
        </CardContent>
      </Card>

      <Separator />

      <Card>
        <CardHeader>
          <CardTitle>File Scanner</CardTitle>
          <CardDescription>
            Click to start scanning and indexing files based on the rules defined above. 
            Note that you still need to select a root folder to start the scan from.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <ScanButton />
        </CardContent>
      </Card>
    </div>
  );
}