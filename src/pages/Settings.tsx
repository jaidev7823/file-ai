/**
 * Settings Component - Manages file scanning configuration
 * 
 * React Concepts Used:
 * - useState: Manages component state (scanSettings, newPath, etc.)
 * - useEffect: Handles side effects like loading data when component mounts
 * - Event Handlers: Functions that respond to user interactions (onClick, onChange)
 * - Conditional Rendering: Shows/hides UI elements based on state
 * - Component Props: Passes data to child components (ScanButton)
 */

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { X, Plus, Folder, Settings as SettingsIcon, Trash2 } from "lucide-react";
import { Separator } from "@/components/ui/separator";
import ScanButton from "@/components/ScanButton";

// TypeScript interface defines the shape of our settings data
interface ScanSettings {
  scanPaths: string[];
  ignoredFolders: string[];
}


export default function Settings() {
  // React Hook: useState manages component state
  // State persists between re-renders and triggers UI updates when changed
  const [scanSettings, setScanSettings] = useState<ScanSettings>({
    scanPaths: [],
    ignoredFolders: [
      "node_modules",
      ".venv",
      "ComfyUI",
      "Adobe",
      ".git",
      "target",
      "build",
      "dist"
    ]
  });

  // Additional state variables for form inputs and loading status
  const [newPath, setNewPath] = useState("");
  const [newIgnoreFolder, setNewIgnoreFolder] = useState("");
  const [loading, setLoading] = useState(false);

  // React Hook: use
  // Empty dependency array [] means this runs only once when component mounts
  useEffect(() => {
    loadSettings();
  }, []);

  // Async function to load initial settings
  // In React, we separate data fetching logic from UI rendering
  const loadSettings = async () => {
    try {
      // TODO: Replace with actual storage/API call
      const defaultPaths = ["C://Users/Jai Mishra/OneDrive/Documents"];
      // setScanSettings uses functional update to merge new data with existing state
      setScanSettings(prev => ({ ...prev, scanPaths: defaultPaths }));
    } catch (error) {
      console.error("Failed to load settings:", error);
    }
  };

  // Event Handler: Adds a new scan path to the settings
  // React Concept: Immutable state updates using spread operator
  const addScanPath = async () => {
    if (!newPath.trim()) return;

    try {
      // State update pattern: Use functional update to avoid race conditions
      // prev => creates new object instead of mutating existing state
      setScanSettings(prev => ({
        ...prev, // Spread existing properties
        scanPaths: [...prev.scanPaths, newPath.trim()] // Create new array with added item
      }));
      setNewPath(""); // Clear input field after adding
    } catch (error) {
      console.error("Failed to add scan path:", error);
    }
  };

  // Event Handler: Removes a scan path from settings
  // React Concept: Array filtering to remove items immutably
  const removeScanPath = (pathToRemove: string) => {
    setScanSettings(prev => ({
      ...prev,
      scanPaths: prev.scanPaths.filter(path => path !== pathToRemove) // Filter creates new array
    }));
  };

  // Event Handler: Adds folder to ignore list
  // React Concept: Input validation and duplicate prevention
  const addIgnoreFolder = () => {
    const folderName = newIgnoreFolder.trim().replace(/^#/, ""); // Clean input
    if (!folderName) return;

    // Prevent duplicates before updating state
    if (!scanSettings.ignoredFolders.includes(folderName)) {
      setScanSettings(prev => ({
        ...prev,
        ignoredFolders: [...prev.ignoredFolders, folderName]
      }));
    }
    setNewIgnoreFolder(""); // Reset form input
  };

  // Event Handler: Removes folder from ignore list
  const removeIgnoreFolder = (folderToRemove: string) => {
    setScanSettings(prev => ({
      ...prev,
      ignoredFolders: prev.ignoredFolders.filter(folder => folder !== folderToRemove)
    }));
  };

  // Async Event Handler: Opens native folder picker dialog
  // React Concept: Integration with external APIs (Tauri invoke)
  const selectFolder = async () => {
    try {
      const selectedPath = await invoke<string>("select_folder");
      // Only add if path exists and isn't already in the list
      if (selectedPath && !scanSettings.scanPaths.includes(selectedPath)) {
        setScanSettings(prev => ({
          ...prev,
          scanPaths: [...prev.scanPaths, selectedPath]
        }));
      }
    } catch (error) {
      console.error("Failed to select folder:", error);
    }
  };

  // Async Event Handler: Saves settings to backend
  // React Concept: Loading states for better UX
  const saveSettings = async () => {
    setLoading(true); // Show loading state
    try {
      // Call backend API to persist settings
      await invoke("save_scan_settings", { settings: scanSettings });
      console.log("Settings saved successfully");
    } catch (error) {
      console.error("Failed to save settings:", error);
    } finally {
      setLoading(false); // Always reset loading state
    }
  };

  return (
    <div className="p-6 max-w-4xl mx-auto space-y-6">
      <div className="flex items-center gap-2 mb-6">
        <SettingsIcon className="h-6 w-6" />
        <h1 className="text-2xl font-bold">Settings</h1>
      </div>

      {/* Scan Paths Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Folder className="h-5 w-5" />
            Scan Paths
          </CardTitle>
          <CardDescription>
            Configure which folders to scan for files. The app will search through all files in these locations.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Current Scan Paths */}
          <div className="space-y-2">
            {scanSettings.scanPaths.map((path, index) => (
              <div key={index} className="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
                <div className="flex items-center gap-2 flex-1 min-w-0">
                  <Folder className="h-4 w-4 text-gray-500 flex-shrink-0" />
                  <span className="text-sm font-mono truncate">{path}</span>
                </div>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => removeScanPath(path)}
                  className="text-red-500 hover:text-red-700 flex-shrink-0"
                >
                  <X className="h-4 w-4" />
                </Button>
              </div>
            ))}
          </div>

          {/* Add New Path */}
          <div className="flex gap-2">
            <Input
              placeholder="Enter folder path or click Browse..."
              value={newPath}
              onChange={(e) => setNewPath(e.target.value)}
              onKeyPress={(e) => e.key === "Enter" && addScanPath()}
              className="flex-1"
            />
            <Button onClick={selectFolder} variant="outline">
              Browse
            </Button>
            <Button onClick={addScanPath} disabled={!newPath.trim()}>
              <Plus className="h-4 w-4" />
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Ignored Folders Configuration */}
      <Card>
        <CardHeader>
          <CardTitle>Ignored Folders</CardTitle>
          <CardDescription>
            Folders that will be skipped during scanning. Add folder names using # syntax (e.g., #node_modules).
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Current Ignored Folders */}
          <div className="flex flex-wrap gap-2">
            {scanSettings.ignoredFolders.map((folder, index) => (
              <Badge key={index} variant="secondary" className="flex items-center gap-1">
                #{folder}
                <button
                  onClick={() => removeIgnoreFolder(folder)}
                  className="ml-1 hover:text-red-500"
                >
                  <X className="h-3 w-3" />
                </button>
              </Badge>
            ))}
          </div>

          {/* Add New Ignored Folder */}
          <div className="flex gap-2">
            <Input
              placeholder="Type folder name (e.g., node_modules or #node_modules)"
              value={newIgnoreFolder}
              onChange={(e) => setNewIgnoreFolder(e.target.value)}
              onKeyPress={(e) => e.key === "Enter" && addIgnoreFolder()}
              className="flex-1"
            />
            <Button onClick={addIgnoreFolder} disabled={!newIgnoreFolder.trim()}>
              <Plus className="h-4 w-4" />
            </Button>
          </div>

          <div className="text-xs text-gray-500">
            <strong>Default ignored folders:</strong> node_modules, .venv, .git, target, build, dist, ComfyUI, Adobe
          </div>
        </CardContent>
      </Card>

      <Separator />

      {/* File Scanner Section */}
      <Card>
        <CardHeader>
          <CardTitle>File Scanner</CardTitle>
          <CardDescription>
            Scan and index files from your configured paths with the current settings.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <ScanButton scanPaths={scanSettings.scanPaths} ignoredFolders={scanSettings.ignoredFolders} />
        </CardContent>
      </Card>

      {/* Save Settings */}
      <div className="flex justify-end">
        <Button onClick={saveSettings} disabled={loading}>
          {loading ? "Saving..." : "Save Settings"}
        </Button>
      </div>
    </div>
  );
}
