import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Backend types from Rust
interface BackendFile {
  id: number;
  name: string;
  extension: string;
  path: string;
  content: string;
  created_at: string;
  updated_at: string;
}

interface BackendSearchResult {
  file: BackendFile;
  relevance_score: number;
  match_type: 'Vector' | 'Text' | { Hybrid: [number, number] };
  snippet?: string;
}

// Frontend types
export interface SearchResult {
  id: string;
  title: string;
  path: string;
  snippet?: string;
  type: 'file' | 'folder' | 'content';
  relevance_score?: number;
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
      let backendResults: BackendSearchResult[] = [];
      
      try {
        // Use the new search_indexed_files command
        backendResults = await invoke<BackendSearchResult[]>('search_indexed_files', { 
          query,
          limit: 10 // Limit results to 10
        });
      } catch (indexedSearchError) {
        console.warn('Indexed search failed, trying hybrid search:', indexedSearchError);
        
        try {
          // Fallback to hybrid search
          backendResults = await invoke<BackendSearchResult[]>('search_files', { 
            query,
            top_k: 10,
            filters: null
          });
        } catch (hybridError) {
          console.warn('Hybrid search failed, trying FTS search:', hybridError);
        }
      }

      // Transform backend results to frontend format
      const transformedResults: SearchResult[] = backendResults.map((result) => ({
        id: result.file.id.toString(),
        title: result.file.name,
        path: result.file.path,
        snippet: result.snippet || generateSnippet(result.file.content, query),
        type: 'file', // All results are files for now
        relevance_score: result.relevance_score
      }));

      setResults(transformedResults);
    } catch (err) {
      console.error('Search error:', err);
      setError(err instanceof Error ? err.message : 'Search failed');
      setResults([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Helper function to generate snippet from content
  const generateSnippet = (content: string, query: string): string => {
    if (!content || !query) return '';
    
    const lowerContent = content.toLowerCase();
    const lowerQuery = query.toLowerCase();
    const index = lowerContent.indexOf(lowerQuery);
    
    if (index === -1) {
      // If query not found, return first 100 characters
      return content.substring(0, 100) + (content.length > 100 ? '...' : '');
    }
    
    // Extract snippet around the match
    const start = Math.max(0, index - 50);
    const end = Math.min(content.length, index + query.length + 50);
    const snippet = content.substring(start, end);
    
    return (start > 0 ? '...' : '') + snippet + (end < content.length ? '...' : '');
  };

  return {
    results,
    isLoading,
    error,
    search
  };
}