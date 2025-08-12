import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { X, Plus, Settings as SettingsIcon } from "lucide-react";
import { Separator } from "@/components/ui/separator";
import ScanButton from "@/components/ScanButton";

// Type definitions
interface ScanSettings {
  includedExtensions: string[];
  excludedFolders: string[];
  includedFolders: string[];
  includedPaths: string[];
  excludedPaths: string[];
  excludedExtensions: string[];
  excludedFilenames: string[];
}

interface RuleConfig {
  key: keyof ScanSettings;
  title: string;
  placeholder: string;
  addCommand: string;
  removeCommand: string;
  color: "green" | "red";
  paramName?: string; // For different parameter names in backend
}

// Configuration for all rule types
const RULE_CONFIGS: RuleConfig[] = [
  {
    key: "includedPaths",
    title: "Included Paths",
    placeholder: "Add included path...",
    addCommand: "add_included_path",
    removeCommand: "remove_included_path",
    color: "green",
  },
  {
    key: "excludedPaths", 
    title: "Excluded Paths",
    placeholder: "Add excluded path...",
    addCommand: "add_excluded_path",
    removeCommand: "remove_excluded_path",
    color: "red",
  },
  {
    key: "includedFolders",
    title: "Included Folders", 
    placeholder: "Add included folder...",
    addCommand: "add_included_folder",
    removeCommand: "remove_included_folder",
    color: "green",
    paramName: "folderName",
  },
  {
    key: "excludedFolders",
    title: "Excluded Folders",
    placeholder: "Add excluded folder...",
    addCommand: "add_excluded_folder", 
    removeCommand: "remove_excluded_folder",
    color: "red",
    paramName: "path",
  },
  {
    key: "includedExtensions",
    title: "Included Extensions",
    placeholder: "Add included extension (e.g., tsx, md)",
    addCommand: "add_included_extension",
    removeCommand: "remove_included_extension", 
    color: "green",
  },
  {
    key: "excludedExtensions",
    title: "Excluded Extensions",
    placeholder: "Add excluded extension...",
    addCommand: "add_excluded_extension",
    removeCommand: "remove_excluded_extension",
    color: "red",
  },
  {
    key: "excludedFilenames",
    title: "Excluded Filenames",
    placeholder: "Add excluded filename...", 
    addCommand: "add_excluded_filename",
    removeCommand: "remove_excluded_filename",
    color: "red",
  },
];

// Backend command mapping
const BACKEND_COMMANDS = {
  get_included_extensions: "get_included_extensions",
  get_excluded_folders: "get_excluded_folders", 
  get_included_folders: "get_included_folders",
  get_included_paths: "get_included_paths",
  get_excluded_paths: "get_excluded_paths",
  get_excluded_extensions: "get_excluded_extensions",
  get_excluded_filenames: "get_excluded_filenames",
};

export default function Settings() {
  const [scanSettings, setScanSettings] = useState<ScanSettings>({
    includedExtensions: [],
    excludedFolders: [],
    includedFolders: [],
    includedPaths: [],
    excludedPaths: [],
    excludedExtensions: [],
    excludedFilenames: [],
  });

  const [inputValues, setInputValues] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);
  const [showAllExtensions, setShowAllExtensions] = useState(false);
  const [fileCount, setFileCount] = useState<number | null>(null);
  const [countLoading, setCountLoading] = useState(false);

  const EXTENSION_LIMIT = 10;

  // Load all settings from backend
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
          excludedFilenames,
        ] = await Promise.all([
          invoke<string[]>(BACKEND_COMMANDS.get_included_extensions),
          invoke<string[]>(BACKEND_COMMANDS.get_excluded_folders),
          invoke<string[]>(BACKEND_COMMANDS.get_included_folders),
          invoke<string[]>(BACKEND_COMMANDS.get_included_paths),
          invoke<string[]>(BACKEND_COMMANDS.get_excluded_paths),
          invoke<string[]>(BACKEND_COMMANDS.get_excluded_extensions),
          invoke<string[]>(BACKEND_COMMANDS.get_excluded_filenames),
        ]);

        setScanSettings({
          includedExtensions,
          excludedFolders,
          includedFolders,
          includedPaths,
          excludedPaths,
          excludedExtensions,
          excludedFilenames,
        });
      } catch (error) {
        console.error("Failed to load settings:", error);
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

  // Generic add function
  const addRule = async (config: RuleConfig) => {
    const inputKey = config.key;
    const value = inputValues[inputKey]?.trim();
    if (!value) return;

    // Clean extension format
    const cleanValue = config.key.includes("Extension") 
      ? value.replace(/^\./, "") 
      : value;

    try {
      const paramName = config.paramName || getParamName(config.key);
      await invoke(config.addCommand, { [paramName]: cleanValue });
      
      setScanSettings(prev => ({
        ...prev,
        [config.key]: [...prev[config.key], cleanValue],
      }));
      
      setInputValues(prev => ({ ...prev, [inputKey]: "" }));
    } catch (error) {
      console.error(`Failed to add ${config.title.toLowerCase()}:`, error);
    }
  };

  // Generic remove function
  const removeRule = async (config: RuleConfig, valueToRemove: string) => {
    try {
      const paramName = config.paramName || getParamName(config.key);
      await invoke(config.removeCommand, { [paramName]: valueToRemove });
      
      setScanSettings(prev => ({
        ...prev,
        [config.key]: prev[config.key].filter(item => item !== valueToRemove),
      }));
    } catch (error) {
      console.error(`Failed to remove ${config.title.toLowerCase()}:`, error);
    }
  };

  // Helper function to get parameter name for backend commands
  const getParamName = (key: keyof ScanSettings): string => {
    const paramMap: Record<keyof ScanSettings, string> = {
      includedPaths: "path",
      excludedPaths: "path", 
      includedFolders: "folderName",
      excludedFolders: "path",
      includedExtensions: "extension",
      excludedExtensions: "extension",
      excludedFilenames: "filename",
    };
    return paramMap[key];
  };

  // Render a rule section
  const renderRuleSection = (config: RuleConfig) => {
    const values = scanSettings[config.key];
    const isExtensions = config.key === "includedExtensions";
    const displayedValues = isExtensions && !showAllExtensions 
      ? values.slice(0, EXTENSION_LIMIT) 
      : values;

    return (
      <div key={config.key}>
        <h3 className={`text-sm font-semibold ${config.color === 'green' ? 'text-green-600' : 'text-red-600'}`}>
          {config.title}
        </h3>
        <div className="flex flex-wrap gap-2 mt-2">
          {displayedValues.length === 0 ? (
            <p className="text-sm text-gray-500">No {config.title.toLowerCase()}</p>
          ) : (
            displayedValues.map((value: string, index: number) => (
              <Badge
                key={index}
                variant="secondary"
                className="flex items-center gap-1"
              >
                {config.key.includes("Extension") ? `.${value}` : value}
                <button
                  onClick={() => removeRule(config, value)}
                  className="ml-1"
                >
                  <X className="h-3 w-3" />
                </button>
              </Badge>
            ))
          )}
          {isExtensions && values.length > EXTENSION_LIMIT && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowAllExtensions(!showAllExtensions)}
            >
              {showAllExtensions ? "Hide" : `Show All (${values.length})`}
            </Button>
          )}
        </div>
        <div className="flex gap-2 mt-2">
          <Input
            value={inputValues[config.key] || ""}
            onChange={(e) => setInputValues(prev => ({ ...prev, [config.key]: e.target.value }))}
            placeholder={config.placeholder}
            className="flex-1"
          />
          <Button 
            variant={config.color === "green" ? "default" : "secondary"} 
            onClick={() => addRule(config)}
          >
            <Plus className="h-4 w-4 mr-1" /> Add
          </Button>
        </div>
      </div>
    );
  };

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-8">
      {/* Header */}
      <div className="flex items-center gap-3">
        <SettingsIcon className="h-7 w-7 text-primary" />
        <h1 className="text-3xl font-bold">File Indexing Settings</h1>
      </div>

      {loading && <div className="text-sm text-gray-500">Loading settings...</div>}

      {/* Path Rules */}
      <Card>
        <CardHeader>
          <CardTitle>Path Rules</CardTitle>
          <CardDescription>Control which specific file paths are included or excluded.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {renderRuleSection(RULE_CONFIGS[0])}
          <Separator />
          {renderRuleSection(RULE_CONFIGS[1])}
        </CardContent>
      </Card>

      {/* Folder Rules */}
      <Card>
        <CardHeader>
          <CardTitle>Folder Rules</CardTitle>
          <CardDescription>Include or exclude folders from scanning.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {renderRuleSection(RULE_CONFIGS[2])}
          <Separator />
          {renderRuleSection(RULE_CONFIGS[3])}
        </CardContent>
      </Card>

      {/* Extension Rules */}
      <Card>
        <CardHeader>
          <CardTitle>Extension Rules</CardTitle>
          <CardDescription>Manage file extensions to include or exclude.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {renderRuleSection(RULE_CONFIGS[4])}
          <Separator />
          {renderRuleSection(RULE_CONFIGS[5])}
        </CardContent>
      </Card>

      {/* Filename Rules */}
      <Card>
        <CardHeader>
          <CardTitle>Filename Rules</CardTitle>
          <CardDescription>Exclude files by exact name or pattern.</CardDescription>
        </CardHeader>
        <CardContent>
          {renderRuleSection(RULE_CONFIGS[6])}
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