import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Match the Rust SearchResult struct exactly
interface BackendSearchResult {
  id: string;
  result_type: string;
  title: string;
  path: string;
  relevance_score: number;
  match_type: 'Vector' | 'Text' | { Hybrid: [number, number] };
  snippet?: string;
}

// Frontend types
export interface SearchResult {
  id: string;
  result_type: string;
  title: string;
  path: string;
  relevance_score?: number;
  type: 'file' | 'folder' | 'content';
  snippet?: string;
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
      // The backend now uses a unified async search command.
      const backendResults = await invoke<BackendSearchResult[]>('search_indexed_files', {
        query,
        limit: 10,
      });

      // Transform backend results to frontend format
      const transformedResults: SearchResult[] = backendResults.map((result) => {
        // Determine match type string
        let matchTypeStr = 'text';
        if (typeof result.match_type === 'string') {
          matchTypeStr = result.match_type.toLowerCase();
        } else if (result.match_type && typeof result.match_type === 'object' && 'Hybrid' in result.match_type) {
          matchTypeStr = 'hybrid';
        }

        // Determine the type based on result_type from backend
        let type: 'file' | 'folder' | 'content' = 'file';
        if (result.result_type === 'folder') {
          type = 'folder';
        } else if (result.result_type === 'content') {
          type = 'content';
        }

        return {
          id: result.id,
          result_type: matchTypeStr,
          title: result.title,
          path: result.path,
          snippet: result.snippet || generateSnippet(result.title, query), // Use title as fallback for snippet generation
          type: type,
          relevance_score: result.relevance_score
        };
      });

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