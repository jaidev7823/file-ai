// types/index.ts

export interface FileResult {
  name: string;
  path: string;
  type: string;
  snippet?: string;
  score?: number;
}

export interface File {
  id?: number;
  name: string;
  extension: string;
  path: string;
  content: string; // Base64 encoded if binary
  createdAt?: string;
  updatedAt?: string;
}
