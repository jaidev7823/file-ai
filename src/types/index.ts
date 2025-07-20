/**
 * Represents a file result from the file scanner
 */
export interface FileResult {
  name: string;      // Name of the file
  path: string;      // Path to the file
  type: string;      // File type (extension or category)
  snippet?: string;  // Optional text snippet from the file
  score?: number;    // Optional relevance score (0-1)
}