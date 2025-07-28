import { useState, useEffect, useRef } from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Search, X, File, Folder, FileText, ExternalLink, FolderOpen, MoreHorizontal } from "lucide-react";
import { useSearch } from "@/hooks/useSearch";
import { invoke } from "@tauri-apps/api/core";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

export default function SearchPage() {
    const [query, setQuery] = useState("");
    const inputRef = useRef<HTMLInputElement>(null);
    const { results, isLoading, error, search } = useSearch();

    // Focus input when component mounts and set transparent background
    useEffect(() => {
        if (inputRef.current) {
            inputRef.current.focus();
        }

        // Make body transparent for search window
        document.body.style.backgroundColor = 'transparent';
        document.documentElement.style.backgroundColor = 'transparent';

        // Cleanup on unmount
        return () => {
            document.body.style.backgroundColor = '';
            document.documentElement.style.backgroundColor = '';
        };
    }, []);

    // Handle escape key to close window
    useEffect(() => {
        const handleEscape = (e: KeyboardEvent) => {
            if (e.key === "Escape") {
                invoke("hide_search_window");
            }
        };

        document.addEventListener("keydown", handleEscape);
        return () => document.removeEventListener("keydown", handleEscape);
    }, []);

    // Handle search with debouncing
    useEffect(() => {
        const timeoutId = setTimeout(() => {
            search(query);
        }, 300);

        return () => clearTimeout(timeoutId);
    }, [query, search]);

    const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setQuery(e.target.value);
    };

    const getIcon = (type: string) => {
        switch (type) {
            case 'folder':
                return <Folder className="h-4 w-4" />;
            case 'content':
                return <FileText className="h-4 w-4" />;
            default:
                return <File className="h-4 w-4" />;
        }
    };

    const handleClose = () => {
        invoke("hide_search_window");
    };

    const handleOpenFile = async (filePath: string) => {
        try {
            await invoke("open_file", { filePath });
            handleClose();
        } catch (error) {
            console.error("Failed to open file:", error);
        }
    };

    const handleOpenWith = async (filePath: string, application: string) => {
        try {
            await invoke("open_file_with", { filePath, application });
            handleClose();
        } catch (error) {
            console.error("Failed to open file with application:", error);
        }
    };

    const handleShowInExplorer = async (filePath: string) => {
        try {
            await invoke("show_file_in_explorer", { filePath });
        } catch (error) {
            console.error("Failed to show file in explorer:", error);
        }
    };

    return (
        <div className="w-full h-screen flex items-start justify-center  bg-transparent">
            <div 
                className="w-full max-w-2xl rounded-lg overflow-hidden search-container" 
                style={{ 
                    backgroundColor: 'rgba(255, 255, 255, 0.95)',
                    backdropFilter: 'blur(12px)',
                    boxShadow: 'none',
                    border: 'none',
                    outline: 'none'
                }}
            >
                <div className="p-4">
                    {/* Search Input */}
                    <div className="relative">
                        <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 h-4 w-4" />
                        <Input
                            ref={inputRef}
                            value={query}
                            onChange={handleInputChange}
                            placeholder="Search files, folders, and content..."
                            className="pl-10 pr-10 h-12 text-lg border-0 bg-transparent focus-visible:ring-0 focus-visible:ring-offset-0"
                        />
                        <button
                            onClick={handleClose}
                            className="absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-400 hover:text-gray-600 transition-colors"
                        >
                            <X className="h-4 w-4" />
                        </button>
                    </div>

                    {/* Loading */}
                    {isLoading && (
                        <div className="mt-4 p-4 text-center text-gray-500">
                            <Search className="h-6 w-6 mx-auto mb-2 animate-spin" />
                            <p>Searching...</p>
                        </div>
                    )}

                    {/* Error */}
                    {error && (
                        <div className="mt-4 p-4 text-center text-red-500">
                            <p>Error: {error}</p>
                        </div>
                    )}

                    {/* Results */}
                    {!isLoading && results.length > 0 && (
                        <div className="mt-4 max-h-80 overflow-y-auto">
                            <div className="space-y-1">
                                {results.map((result) => (
                                    <div
                                        key={result.id}
                                        className="p-3 rounded-lg hover:bg-gray-100 transition-colors flex items-start gap-3 group"
                                    >
                                        <div className="flex-shrink-0 mt-0.5 text-gray-500">
                                            {getIcon(result.type)}
                                        </div>
                                        <div 
                                            className="flex-1 min-w-0 cursor-pointer"
                                            onClick={() => handleOpenFile(result.path)}
                                        >
                                            <div className="text-sm font-medium text-gray-900 truncate">
                                                {result.title}
                                            </div>
                                            <div className="text-xs text-gray-500 truncate">
                                                {result.path}
                                            </div>
                                            {result.snippet && (
                                                <div className="text-xs text-gray-600 mt-1 line-clamp-2">
                                                    {result.snippet}
                                                </div>
                                            )}
                                        </div>
                                        <div className="flex-shrink-0 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                                            <Button
                                                size="sm"
                                                variant="ghost"
                                                className="h-7 w-7 p-0"
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    handleOpenFile(result.path);
                                                }}
                                                title="Open file"
                                            >
                                                <ExternalLink className="h-3 w-3" />
                                            </Button>
                                            <Button
                                                size="sm"
                                                variant="ghost"
                                                className="h-7 w-7 p-0"
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    handleShowInExplorer(result.path);
                                                }}
                                                title="Show in explorer"
                                            >
                                                <FolderOpen className="h-3 w-3" />
                                            </Button>
                                            <DropdownMenu>
                                                <DropdownMenuTrigger asChild>
                                                    <Button
                                                        size="sm"
                                                        variant="ghost"
                                                        className="h-7 w-7 p-0"
                                                        onClick={(e) => e.stopPropagation()}
                                                        title="Open with..."
                                                    >
                                                        <MoreHorizontal className="h-3 w-3" />
                                                    </Button>
                                                </DropdownMenuTrigger>
                                                <DropdownMenuContent align="end">
                                                    <DropdownMenuItem
                                                        onClick={() => handleOpenWith(result.path, "notepad")}
                                                    >
                                                        Open with Notepad
                                                    </DropdownMenuItem>
                                                    <DropdownMenuItem
                                                        onClick={() => handleOpenWith(result.path, "code")}
                                                    >
                                                        Open with VS Code
                                                    </DropdownMenuItem>
                                                    <DropdownMenuItem
                                                        onClick={() => handleOpenWith(result.path, "notepad++")}
                                                    >
                                                        Open with Notepad++
                                                    </DropdownMenuItem>
                                                </DropdownMenuContent>
                                            </DropdownMenu>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}

                    {/* Empty state */}
                    {query && !isLoading && results.length === 0 && !error && (
                        <div className="mt-4 p-8 text-center text-gray-500">
                            <Search className="h-8 w-8 mx-auto mb-2 opacity-50" />
                            <p>No results found for "{query}"</p>
                        </div>
                    )}

                    {/* Help text */}
                    {!query && (
                        <div className="mt-4 p-4 text-center text-gray-400 text-sm">
                            <p>Start typing to search files and content...</p>
                            <p className="mt-1">Press <kbd className="px-1 py-0.5 bg-gray-100 rounded text-xs">Esc</kbd> to close</p>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}