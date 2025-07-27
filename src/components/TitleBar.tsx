// src/components/TitleBar.tsx
import { getCurrent } from '@tauri-apps/plugin-window';

const currentWindow = getCurrent();

export default function TitleBar() {
  return (
    <div
      className="flex justify-between items-center px-4 py-2 bg-gray-950 text-white select-none"
      data-tauri-drag-region
    >
      <h1 className="text-lg font-semibold">Settings</h1>
      <div className="space-x-2">
        <button
          onClick={() => currentWindow.minimize()}
          className="hover:text-yellow-400"
        >
          🟡
        </button>
        <button
          onClick={() => currentWindow.close()}
          className="hover:text-red-500"
        >
          🔴
        </button>
      </div>
    </div>
  );
}
