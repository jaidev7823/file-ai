// api.ts
import { invoke } from '@tauri-apps/api/core';

// CREATE
export const createFile = async (file: Omit<File, 'id'>) => {
  // The return type here should probably be the newly created File object
  return await invoke<File>('create_file', { file });
};

// READ (All)
export const getFiles = async () => {
  return await invoke<File[]>('get_files');
};

// READ (One)
export const getFileById = async (id: number) => {
  return await invoke<File>('get_file_by_id', { id });
};

// UPDATE
export const updateFile = async (id: number, file: Partial<Omit<File, 'id'>>) => {
  // Usually returns the updated file
  return await invoke<File>('update_file', { id, file });
};

// DELETE
export const deleteFile = async (id: number) => {
  // Delete often returns nothing or a success boolean
  return await invoke<void>('delete_file', { id });
};