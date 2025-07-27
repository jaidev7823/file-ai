import { useEffect, useState } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import App from './App';
import SearchPage from './pages/SearchPage';

export default function WindowRouter() {
  const [windowLabel, setWindowLabel] = useState<string>('');

  useEffect(() => {
    const getWindowLabel = async () => {
      try {
        const currentWindow = getCurrentWebviewWindow();
        setWindowLabel(currentWindow.label);
      } catch (error) {
        console.error('Failed to get window label:', error);
        setWindowLabel('main'); // fallback to main
      }
    };

    getWindowLabel();
  }, []);

  // Render different components based on window label
  switch (windowLabel) {
    case 'search':
      return <SearchPage />;
    case 'main':
    default:
      return <App />;
  }
}