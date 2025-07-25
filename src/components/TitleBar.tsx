import { getCurrentWindow } from '@tauri-apps/api/window';
import { Maximize2, Minimize2, X, Square } from 'lucide-react';
import { Button } from './ui/button';
import { useEffect, useState } from 'react';

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);
  const currentWindow = getCurrentWindow();

  useEffect(() => {
    const checkMaximized = async () => {
      const maximized = await currentWindow.isMaximized();
      setIsMaximized(maximized);
    };

    checkMaximized();

    const unlistenResize = currentWindow.onResized(() => {
      checkMaximized();
    });

    return () => {
      unlistenResize.then(f => f());
    };
  }, [currentWindow]);

  return (
    <div 
      data-tauri-drag-region 
      className="flex items-center justify-between pl-4 pr-2 py-2 bg-background border-b select-none"
      onDoubleClick={() => currentWindow.toggleMaximize()}
    >
      <h1 className="text-sm font-medium">Your App Name</h1>
      
      <div className="flex items-center gap-1">
        <Button 
          variant="ghost"
          size="icon"
          className="h-8 w-8 rounded-none hover:bg-muted/50"
          onClick={() => currentWindow.minimize()}
          title="Minimize"
        >
          <Minimize2 className="h-3 w-3" />
        </Button>
        
        <Button 
          variant="ghost"
          size="icon"
          className="h-8 w-8 rounded-none hover:bg-muted/50"
          onClick={() => currentWindow.toggleMaximize()}
          title={isMaximized ? "Restore" : "Maximize"}
        >
          {isMaximized ? (
            <Square className="h-3 w-3" />
          ) : (
            <Maximize2 className="h-3 w-3" />
          )}
        </Button>
        
        <Button 
          variant="ghost"
          size="icon"
          className="h-8 w-8 rounded-none hover:bg-destructive hover:text-destructive-foreground"
          onClick={() => currentWindow.close()}
          title="Close"
        >
          <X className="h-3 w-3" />
        </Button>
      </div>
    </div>
  );
}