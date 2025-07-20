import { ReactNode } from 'react';

interface AppLayoutProps {
  children: ReactNode;
}

export function AppLayout({ children }: AppLayoutProps) {
  return (
    <div className="flex h-screen bg-background">
      {/* Sidebar */}
      <div className="w-64 bg-card border-r border-border flex flex-col">
        <div className="p-6 border-b border-border">
          <h1 className="text-xl font-semibold">File Scanner</h1>
        </div>
        <nav className="flex-1 p-4">
          <div className="space-y-2">
            <button className="w-full text-left px-3 py-2 rounded-md bg-primary text-primary-foreground">
              Scanner
            </button>
            <button className="w-full text-left px-3 py-2 rounded-md hover:bg-accent hover:text-accent-foreground">
              Settings
            </button>
            <button className="w-full text-left px-3 py-2 rounded-md hover:bg-accent hover:text-accent-foreground">
              About
            </button>
          </div>
        </nav>
      </div>
      
      {/* Main Content */}
      <div className="flex-1 flex flex-col">
        <header className="bg-card border-b border-border px-6 py-4">
          <h2 className="text-lg font-medium">Text File Scanner</h2>
        </header>
        <main className="flex-1 p-6 overflow-auto">
          {children}
        </main>
      </div>
    </div>
  );
}