import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { X, Plus, Settings as SettingsIcon, FileX, FolderX, File, Sliders } from "lucide-react";
import { Separator } from "@/components/ui/separator";
import ScanButton from "@/components/ScanButton";

interface ScanSettings {
  ignoredExtensions: string[];
  ignoredFolders: string[];
  includedFolders: string[];
  includedPaths: string[];
  excludedPaths: string[];
  excludedExtensions: string[];
  excludedFilenames: string[];
}


export default function Settings() {
  const [scanSettings, setScanSettings] = useState<ScanSettings>({
    ignoredExtensions: [],
    ignoredFolders: [],
    includedFolders: [],
    includedPaths: [],
    excludedPaths: [],
    excludedExtensions: [],
    excludedFilenames: [],
  });
  

  const [newExtension, setNewExtension] = useState("");
  const [newIgnoreFolder, setNewIgnoreFolder] = useState("");
  const [newIncludedPath, setNewIncludedPath] = useState("");
  const [newExcludedPath, setNewExcludedPath] = useState("");
  const [newIncludedFolder, setNewIncludedFolder] = useState("");
  const [newExcludedExtension, setNewExcludedExtension] = useState("");
  const [newExcludedFilename, setNewExcludedFilename] = useState("");

  const [loading, setLoading] = useState(false);
  const [showAllExtensions, setShowAllExtensions] = useState(false);
  const EXTENSION_LIMIT = 10;

  const [fileCount, setFileCount] = useState<number | null>(null);
  const [countLoading, setCountLoading] = useState(false);

  // Load settings from backend
  useEffect(() => {
    const loadAllSettings = async () => {
      setLoading(true);
      try {
        const [
          includedExtensions,
          excludedFolders,
          includedFolders,
          includedPaths,
          excludedPaths,
          excludedExtensions,
          excludedFilenames
        ] = await Promise.all([
          invoke<string[]>("get_included_extensions"),
          invoke<string[]>("get_included_folders"),
          invoke<string[]>("get_included_path"),
          invoke<string[]>("get_excluded_paths"),
          invoke<string[]>("get_excluded_folder"),
          invoke<string[]>("get_excluded_extension"),
          invoke<string[]>("get_excluded_filenames"),
        ]);

        setScanSettings({
          ignoredExtensions: includedExtensions,
          ignoredFolders: excludedFolders,
          includedFolders,
          includedPaths,
          excludedPaths,
          excludedExtensions,
          excludedFilenames,
        });
      } catch (error) {
        console.error("Failed to load settings from database:", error);
      } finally {
        setLoading(false);
      }
    };
    loadAllSettings();
  }, []);


  // File count check
  const fetchFileCount = async () => {
    setCountLoading(true);
    try {
      const count = await invoke<number>("get_matching_file_count");
      setFileCount(count);
    } catch (error) {
      console.error("Failed to get file count:", error);
    } finally {
      setCountLoading(false);
    }
  };

  // Included Extensions
  const addIncludedExtension = async () => {
    const extension = newExtension.trim().replace(/^\./, "");
    if (!extension) return;
    try {
      await invoke("add_included_extension", { extension });
      setScanSettings(prev => ({
        ...prev,
        ignoredExtensions: [...prev.ignoredExtensions, extension]
      }));
      setNewExtension("");
    } catch (error) {
      console.error("Failed to add included extension:", error);
    }
  };

  const removeIncludedExtension = async (extToRemove: string) => {
    try {
      await invoke("remove_included_extension", { extension: extToRemove });
      setScanSettings(prev => ({
        ...prev,
        ignoredExtensions: prev.ignoredExtensions.filter(ext => ext !== extToRemove)
      }));
    } catch (error) {
      console.error("Failed to remove included extension:", error);
    }
  };

  // Excluded Folders
  const addExcludedFolder = async () => {
    const folderName = newIgnoreFolder.trim();
    if (!folderName) return;
    try {
      await invoke("add_excluded_folder", { folder: folderName });
      setScanSettings(prev => ({
        ...prev,
        ignoredFolders: [...prev.ignoredFolders, folderName]
      }));
      setNewIgnoreFolder("");
    } catch (error) {
      console.error("Failed to add excluded folder:", error);
    }
  };

  const removeExcludedFolder = async (folderToRemove: string) => {
    try {
      await invoke("remove_excluded_folder", { folder: folderToRemove });
      setScanSettings(prev => ({
        ...prev,
        ignoredFolders: prev.ignoredFolders.filter(folder => folder !== folderToRemove)
      }));
    } catch (error) {
      console.error("Failed to remove excluded folder:", error);
    }
  };

  // Path Rules
  const addIncludedPath = async () => {
    if (!newIncludedPath.trim()) return;
    try {
      await invoke("add_included_path", { path: newIncludedPath.trim() });
      setNewIncludedPath("");
    } catch (error) {
      console.error("Failed to add included path:", error);
    }
  };

  const addExcludedPath = async () => {
    if (!newExcludedPath.trim()) return;
    try {
      await invoke("add_excluded_path", { path: newExcludedPath.trim() });
      setNewExcludedPath("");
    } catch (error) {
      console.error("Failed to add excluded path:", error);
    }
  };

  // Folder Rules
  const addIncludedFolder = async () => {
    if (!newIncludedFolder.trim()) return;
    try {
      await invoke("add_included_folder", { folder: newIncludedFolder.trim() });
      setNewIncludedFolder("");
    } catch (error) {
      console.error("Failed to add included folder:", error);
    }
  };

  // Excluded Extensions
  const addExcludedExtension = async () => {
    if (!newExcludedExtension.trim()) return;
    try {
      await invoke("add_excluded_extension", { extension: newExcludedExtension.trim() });
      setNewExcludedExtension("");
    } catch (error) {
      console.error("Failed to add excluded extension:", error);
    }
  };

  // Filename Rules
  const addExcludedFilename = async () => {
    if (!newExcludedFilename.trim()) return;
    try {
      await invoke("add_excluded_filename", { filename: newExcludedFilename.trim() });
      setNewExcludedFilename("");
    } catch (error) {
      console.error("Failed to add excluded filename:", error);
    }
  };

  const displayedExtensions = showAllExtensions
    ? scanSettings.ignoredExtensions
    : scanSettings.ignoredExtensions.slice(0, EXTENSION_LIMIT);

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-8">
      {/* Header */}
      <div className="flex items-center gap-3">
        <SettingsIcon className="h-7 w-7 text-primary" />
        <h1 className="text-3xl font-bold">File Indexing Settings</h1>
      </div>

      {/* Path Rules */}
      <Card>
        <CardHeader>
          <CardTitle>Path Rules</CardTitle>
          <CardDescription>Control which specific file paths are included or excluded.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div>
            <h3 className="text-sm font-semibold text-green-600">Included Paths</h3>
            <div className="flex gap-2 mt-2">
              <Input value={newIncludedPath} onChange={e => setNewIncludedPath(e.target.value)} placeholder="Add included path..." className="flex-1" />
              <Button variant="secondary" onClick={addIncludedPath}>
                <Plus className="h-4 w-4 mr-1" /> Add
              </Button>
            </div>
          </div>
          <Separator />
          <div>
            <h3 className="text-sm font-semibold text-red-600">Excluded Paths</h3>
            <div className="flex gap-2 mt-2">
              <Input value={newExcludedPath} onChange={e => setNewExcludedPath(e.target.value)} placeholder="Add excluded path..." className="flex-1" />
              <Button variant="secondary" onClick={addExcludedPath}>
                <Plus className="h-4 w-4 mr-1" /> Add
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Folder Rules */}
      <Card>
        <CardHeader>
          <CardTitle>Folder Rules</CardTitle>
          <CardDescription>Include or exclude folders from scanning.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div>
            <h3 className="text-sm font-semibold text-green-600">Included Folders</h3>
            <div className="flex gap-2 mt-2">
              <Input value={newIncludedFolder} onChange={e => setNewIncludedFolder(e.target.value)} placeholder="Add included folder..." className="flex-1" />
              <Button variant="secondary" onClick={addIncludedFolder}>
                <Plus className="h-4 w-4 mr-1" /> Add
              </Button>
            </div>
          </div>
          <Separator />
          <div>
            <h3 className="text-sm font-semibold text-red-600">Excluded Folders</h3>
            <div className="flex flex-wrap gap-2 mt-2">
              {scanSettings.ignoredFolders.map((folder, index) => (
                <Badge key={index} variant="secondary" className="flex items-center gap-1">
                  {folder}
                  <button onClick={() => removeExcludedFolder(folder)} className="ml-1">
                    <X className="h-3 w-3" />
                  </button>
                </Badge>
              ))}
            </div>
            <div className="flex gap-2 mt-2">
              <Input value={newIgnoreFolder} onChange={e => setNewIgnoreFolder(e.target.value)} placeholder="Add excluded folder..." className="flex-1" />
              <Button onClick={addExcludedFolder}>
                <Plus className="h-4 w-4 mr-1" /> Add
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Extension Rules */}
      <Card>
        <CardHeader>
          <CardTitle>Extension Rules</CardTitle>
          <CardDescription>Manage file extensions to include or exclude.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div>
            <h3 className="text-sm font-semibold text-green-600">Included Extensions</h3>
            <div className="flex flex-wrap gap-2 mt-2">
              {displayedExtensions.map((ext, index) => (
                <Badge key={index} variant="secondary" className="flex items-center gap-1">
                  .{ext}
                  <button onClick={() => removeIncludedExtension(ext)} className="ml-1">
                    <X className="h-3 w-3" />
                  </button>
                </Badge>
              ))}
              {scanSettings.ignoredExtensions.length > EXTENSION_LIMIT && (
                <Button variant="ghost" size="sm" onClick={() => setShowAllExtensions(!showAllExtensions)}>
                  {showAllExtensions ? "Hide" : `Show All (${scanSettings.ignoredExtensions.length})`}
                </Button>
              )}
            </div>
            <div className="flex gap-2 mt-2">
              <Input value={newExtension} onChange={e => setNewExtension(e.target.value)} placeholder="Add included extension (e.g., tsx, md)" className="flex-1" />
              <Button onClick={addIncludedExtension}>
                <Plus className="h-4 w-4 mr-1" /> Add
              </Button>
            </div>
          </div>
          <Separator />
          <div>
            <h3 className="text-sm font-semibold text-red-600">Excluded Extensions</h3>
            <div className="flex gap-2 mt-2">
              <Input value={newExcludedExtension} onChange={e => setNewExcludedExtension(e.target.value)} placeholder="Add excluded extension..." className="flex-1" />
              <Button variant="secondary" onClick={addExcludedExtension}>
                <Plus className="h-4 w-4 mr-1" /> Add
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Filename Rules */}
      <Card>
        <CardHeader>
          <CardTitle>Filename Rules</CardTitle>
          <CardDescription>Exclude files by exact name or pattern.</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex gap-2 mt-2">
            <Input value={newExcludedFilename} onChange={e => setNewExcludedFilename(e.target.value)} placeholder="Add excluded filename..." className="flex-1" />
            <Button variant="secondary" onClick={addExcludedFilename}>
              <Plus className="h-4 w-4 mr-1" /> Add
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* File Stats */}
      <Card>
        <CardHeader>
          <CardTitle>File Stats</CardTitle>
          <CardDescription>See how many files match the current rules before scanning.</CardDescription>
        </CardHeader>
        <CardContent className="flex items-center gap-4">
          <Button onClick={fetchFileCount} disabled={countLoading}>
            {countLoading ? "Checking..." : "Get File Count"}
          </Button>
          {fileCount !== null && (
            <span className="text-sm text-gray-600">
              {fileCount} file{fileCount !== 1 && "s"} match the current rules.
            </span>
          )}
        </CardContent>
      </Card>

      {/* Scanner */}
      <Card>
        <CardHeader>
          <CardTitle>File Scanner</CardTitle>
          <CardDescription>Click to start scanning and indexing based on the rules above.</CardDescription>
        </CardHeader>
        <CardContent>
          <ScanButton />
        </CardContent>
      </Card>
    </div>
  );
}
