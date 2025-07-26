// src/components/Search.tsx
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { Card, CardHeader, CardTitle, CardContent } from './ui/card';

// Define the TypeScript interface for the Rust SearchResult and File structs
// Make sure this matches your Rust struct definitions exactly.
interface File {
  id: number;
  name: string;
  extension: string;
  path: string;
  content: string;
  created_at: string; // Or Date if you parse it
  updated_at: string; // Or Date if you parse it
}

interface SearchResult {
  file: File;
  relevance_score: number; // In Rust it's f32, which maps to number in JS
  // Add other fields if needed, like match_type, snippet
  // match_type: { Vector: number } | { Text: number } | { Hybrid: [number, number] }; // Example
  // snippet?: string;
}

export default function Search() {
  const [query, setQuery] = useState('');
  // Update the type of results to match the Rust SearchResult
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSearch = async () => {
    if (!query.trim()) return; // Prevent search on empty query
    setLoading(true);
    setError(null);
    setResults([]); // Clear previous results
    try {
      // Invoke the search_files command and expect an array of SearchResult objects
      const matches = await invoke<SearchResult[]>('search_files', {
        query: query.trim(), // Trim query before sending
        topK: 5,
      });
      setResults(matches);
    } catch (err: any) {
      setError(err?.toString() || 'Unknown search error');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Semantic File Search</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex gap-2">
          <Input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search files semantically..."
            className="flex-1"
            onKeyDown={(e) => { // Allow pressing Enter to search
              if (e.key === 'Enter') {
                handleSearch();
              }
            }}
          />
          <Button onClick={handleSearch} disabled={loading || !query.trim()}>
            {loading ? 'Searching...' : 'Search'}
          </Button>
        </div>

        {error && <div className="text-red-500 text-sm">{error}</div>}

        {results.length > 0 && (
          <div className="space-y-2">
            {results.map((result, i) => ( // Change destructuring here!
              <div key={i} className="text-sm font-mono truncate">
                {/* Access properties of the result object directly */}
                <strong>{result.relevance_score.toFixed(3)}</strong>: {result.file.path}
              </div>
            ))}
          </div>
        )}

        {!loading && !error && results.length === 0 && query.trim() && (
            <div className="text-center text-muted-foreground py-4">No results found.</div>
        )}
        
      </CardContent>
    </Card>
  );
}