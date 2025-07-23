// src/components/Search.tsx
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { Card, CardHeader, CardTitle, CardContent } from './ui/card';

export default function Search() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<[string, number][]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSearch = async () => {
    if (!query) return;
    setLoading(true);
    setError(null);
    try {
      const matches = await invoke<[string, number][]>('search_files', {
        query,
        topK: 5,
      });
      setResults(matches);
    } catch (err: any) {
      setError(err?.toString());
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
          />
          <Button onClick={handleSearch} disabled={loading}>
            {loading ? 'Searching...' : 'Search'}
          </Button>
        </div>

        {error && <div className="text-red-500 text-sm">{error}</div>}

        {results.length > 0 && (
          <div className="space-y-2">
            {results.map(([path, score], i) => (
              <div key={i} className="text-sm font-mono truncate">
                <strong>{score.toFixed(3)}</strong>: {path}
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
