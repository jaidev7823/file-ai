import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AppLayout } from './components/AppLayout';
import { Button } from './components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from './components/ui/card';
import { Alert, AlertDescription } from './components/ui/alert';
import { Progress } from './components/ui/progress';
import { ScrollArea } from './components/ui/scroll-area';
import { Badge } from './components/ui/badge';
import { Separator } from './components/ui/separator';
import './App.css';

function App() {
  const [files, setFiles] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedIdx, setExpandedIdx] = useState<number | null>(null);
  const [fileContent, setFileContent] = useState<string | null>(null);
  const [contentLoading, setContentLoading] = useState(false);

  const handleScan = async () => {
    setIsLoading(true);
    setError(null);
    setFiles([]);
    setExpandedIdx(null);
    setFileContent(null);
    try {
      // You can change the path below to any directory you want to scan
      const result = await invoke<string[]>(
        'scan_text_files',
        { path: 'C://Users/Jai Mishra/OneDrive/Documents' } // Change this path as needed
      );
      setFiles(result);
    } catch (err: any) {
      setError(err?.toString() || 'Unknown error');
    } finally {
      setIsLoading(false);
    }
  };

  const handleReadContent = async (file: string, idx: number) => {
    if (expandedIdx === idx) {
      // Collapse if already expanded
      setExpandedIdx(null);
      setFileContent(null);
      return;
    }
    setExpandedIdx(idx);
    setContentLoading(true);
    setFileContent(null);
    try {
      const result = await invoke<{ path: string; content: string } | null>(
        'get_file_content',
        { path: file, max_chars: 10000 }
      );
      setFileContent(result?.content || '');
    } catch (err: any) {
      setFileContent('Error reading file content.');
    } finally {
      setContentLoading(false);
    }
  };

  return (
    <AppLayout>
      <div className="space-y-6">
        {/* Scanner Panel */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center justify-between">
              File Scanner
              {files.length > 0 && (
                <Badge variant="secondary">{files.length} files found</Badge>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <Button
              onClick={handleScan}
              disabled={isLoading}
              size="lg"
              className="w-full"
            >
              {isLoading ? 'Scanning...' : 'Scan for Text Files'}
            </Button>

            {isLoading && (
              <div className="space-y-2">
                <Progress value={undefined} className="w-full" />
                <p className="text-sm text-muted-foreground text-center">
                  Scanning documents folder...
                </p>
              </div>
            )}

            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
          </CardContent>
        </Card>

        {/* Results Panel */}
        <Card>
          <CardHeader>
            <CardTitle>Scan Results</CardTitle>
          </CardHeader>
          <CardContent>
            <ScrollArea className="h-96 w-full">
              {files.length > 0 ? (
                <div className="space-y-2">
                  {files.map((file, idx) => (
                    <div key={idx}>
                      <div className="flex items-center justify-between py-2 gap-2">
                        <span className="text-sm font-mono truncate flex-1 mr-2">
                          {file}
                        </span>
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => handleReadContent(file, idx)}
                        >
                          {expandedIdx === idx ? 'Hide Content' : 'Read Content'}
                        </Button>
                      </div>
                      {expandedIdx === idx && (
                        <div className="bg-gray-100 rounded p-3 mt-2 text-xs font-mono whitespace-pre-wrap max-h-60 overflow-auto">
                          {contentLoading ? 'Loading...' : fileContent}
                        </div>
                      )}
                      {idx < files.length - 1 && <Separator />}
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-center py-8 text-muted-foreground">
                  {isLoading ? 'Scanning...' : 'No files scanned yet. Click the scan button to get started.'}
                </div>
              )}
            </ScrollArea>
          </CardContent>
        </Card>
      </div>
    </AppLayout>
  );
}

export default App;