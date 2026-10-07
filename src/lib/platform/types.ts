import type { TraceImage, TraceOverlay } from '../persistence/tracing';

export type FileKind = 'project' | 'presets' | 'trace' | 'png' | 'svg';
export type StateKey = 'recovery' | 'presets' | 'preferences' | 'trace-metadata';
export interface PlatformError {
  code: string;
  message: string;
}
export type FileResult<T> =
  | { status: 'success'; value: T }
  | { status: 'cancelled' }
  | { status: 'error'; error: PlatformError };
export interface SelectedFile {
  token: string;
  name: string;
  path: string;
  mime: string;
}
export interface OpenedFile {
  file: File;
  selected: SelectedFile | null;
}
export type UnsavedChoice = 'save' | 'discard' | 'cancel';

export function platformError(error: unknown): PlatformError {
  if (error && typeof error === 'object' && 'message' in error)
    return { code: 'code' in error ? String(error.code) : 'io', message: String(error.message) };
  return { code: 'io', message: typeof error === 'string' ? error : 'The operation failed.' };
}
export async function fileResult<T>(operation: () => Promise<T>): Promise<FileResult<T>> {
  try {
    return { status: 'success', value: await operation() };
  } catch (error) {
    return { status: 'error', error: platformError(error) };
  }
}

/** Filesystem paths are display-only. Only tokens from a native picker can be written. */
export interface PlatformAdapter {
  desktop: boolean;
  chooseFile(kind: 'project' | 'presets' | 'trace'): Promise<FileResult<OpenedFile>>;
  chooseSaveFile(
    kind: Exclude<FileKind, 'trace'>,
    defaultName: string
  ): Promise<FileResult<SelectedFile>>;
  writeFile(selected: SelectedFile, bytes: Uint8Array): Promise<FileResult<void>>;
  releaseFile(selected: SelectedFile): Promise<void>;
  exportFile(kind: Exclude<FileKind, 'trace'>, blob: Blob, name: string): Promise<FileResult<void>>;
  readState(key: StateKey): Promise<string | null>;
  writeState(key: StateKey, value: string | null): Promise<void>;
  quarantineState(key: StateKey): Promise<void>;
  loadTrace(): Promise<TraceImage | null>;
  saveTrace(trace: TraceOverlay | null): Promise<void>;
  confirmAction(message: string): Promise<boolean>;
  confirmUnsaved(name: string, message?: string): Promise<UnsavedChoice>;
  confirmConflict(): Promise<boolean>;
  setDocumentTitle(name: string, dirty: boolean): Promise<void>;
  onCloseRequested(handler: () => Promise<void>): Promise<() => void>;
  destroyWindow(): Promise<void>;
  flush(): Promise<void>;
}
