// Minimal safe wrappers for Tauri APIs to avoid runtime errors in non-Tauri envs
// Uses lazy dynamic imports to keep Vite dev server happy.

export function isTauriEnvironment(): boolean {
  // Tauri 2 exposes __TAURI_INTERNALS__ and process.env.TAURI_PLATFORM in app env
  // Also check for navigator.userAgent containing "Tauri" as a heuristic
  if (typeof window === 'undefined') return false;
  const anyWindow = window as unknown as { __TAURI_INTERNALS__?: unknown };
  const hasInternals = Boolean(anyWindow.__TAURI_INTERNALS__);
  const uaHasTauri = typeof navigator !== 'undefined' && /Tauri/i.test(navigator.userAgent || '');
  return hasInternals || uaHasTauri;
}

export async function safeInvoke<T = unknown>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriEnvironment()) {
    throw new Error('Tauri is not available in this environment');
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args as Record<string, unknown>);
}

export type UnlistenFn = () => void | Promise<void>;

export async function safeListen<TPayload = unknown>(event: string, handler: (event: { payload: TPayload }) => void): Promise<UnlistenFn> {
  if (!isTauriEnvironment()) {
    // No-op unlisten in non-tauri
    return async () => {};
  }
  const { listen } = await import('@tauri-apps/api/event');
  return listen<TPayload>(event, handler);
}




