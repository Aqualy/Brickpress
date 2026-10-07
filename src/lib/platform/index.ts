import { createBrowserPlatform } from './browser';
import type { PlatformAdapter } from './types';

/** Call after mount; the static/browser build never initializes a native API. */
export async function createPlatform(): Promise<PlatformAdapter> {
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window)
    return (await import('./native')).createNativePlatform();
  return createBrowserPlatform();
}
