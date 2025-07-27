import { useEffect } from 'react';

interface UseGlobalShortcutProps {
  key: string;
  ctrlKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  callback: () => void;
  enabled?: boolean;
}

export function useGlobalShortcut({
  key,
  ctrlKey = false,
  shiftKey = false,
  altKey = false,
  callback,
  enabled = true,
}: UseGlobalShortcutProps) {
  useEffect(() => {
    if (!enabled) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      const isCorrectKey = event.key.toLowerCase() === key.toLowerCase();
      const isCorrectCtrl = ctrlKey ? event.ctrlKey : !event.ctrlKey;
      const isCorrectShift = shiftKey ? event.shiftKey : !event.shiftKey;
      const isCorrectAlt = altKey ? event.altKey : !event.altKey;

      if (isCorrectKey && isCorrectCtrl && isCorrectShift && isCorrectAlt) {
        event.preventDefault();
        event.stopPropagation();
        callback();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [key, ctrlKey, shiftKey, altKey, callback, enabled]);
}