import { useState, useCallback } from 'react';
// import { invoke } from '@tauri-apps/api/core'; // Uncomment when implementing actual search

export interface SearchResult {
  id: string;
  title: string;
  path: string;
  snippet?: string;
  type: 'file' | 'folder' | 'content';
}

export function useSearch() {
  const [results, setResults] = useState<SearchResult[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const search = useCallback(async (query: string) => {
    if (!query.trim()) {
      setResults([]);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      // TODO: Replace with your actual search command
      // const searchResults = await invoke<SearchResult[]>('search_files', { query });
      
      // Mock results for now - replace this with actual Tauri command
      const mockResults: SearchResult[] = [
        {
          id: '1',
          title: `File containing "${query}"`,
          path: '/path/to/file1.txt',
          snippet: `This file contains ${query} in its content...`,
          type: 'file'
        },
        {
          id: '2',
          title: `Folder: ${query}`,
          path: `/path/to/${query}`,
          type: 'folder'
        },
        {
          id: '3',
          title: `Content match: ${query}`,
          path: '/path/to/document.md',
          snippet: `...some text with ${query} highlighted...`,
          type: 'content'
        }
      ];

      setResults(mockResults);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
      setResults([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  return {
    results,
    isLoading,
    error,
    search
  };
}