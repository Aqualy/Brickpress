import { parseDocument } from '../persistence/document';
import { filename } from '../export/export';
import {
  platformError,
  type OpenedFile,
  type PlatformAdapter,
  type SelectedFile,
  type UnsavedChoice
} from './types';

export interface DocumentEditor {
  doc: { name: string };
  dirty: boolean;
  documentBusy: boolean;
  transitioning: boolean;
  activeFilePath: string | null;
  projectText(): string;
  load(text: string, recovered?: boolean): void;
  newDocument(): void;
  notify(message: string): void;
}

/** One command path for toolbar, menu, shortcuts and native window close. */
export class DocumentController {
  activeFile: SelectedFile | null = null;
  lastSaved: string | null = null;
  private cleanSnapshot: string | null;
  closing = false;
  private running = false;
  constructor(
    readonly editor: DocumentEditor,
    readonly platform: PlatformAdapter,
    private flushStorage: (includeRecovery: boolean) => Promise<void>
  ) {
    this.cleanSnapshot = editor.projectText();
  }

  recover(text: string) {
    this.editor.load(text, true);
    this.activeFile = null;
    this.editor.activeFilePath = null;
    this.lastSaved = this.cleanSnapshot = null;
  }
  documentChanged() {
    if (this.platform.desktop)
      this.editor.dirty =
        this.cleanSnapshot === null || this.editor.projectText() !== this.cleanSnapshot;
  }
  private async command(task: () => Promise<boolean>): Promise<boolean> {
    if (this.running) return false;
    this.running = this.editor.documentBusy = true;
    try {
      return await task();
    } catch (error) {
      this.editor.notify(platformError(error).message);
      return false;
    } finally {
      this.running = this.editor.documentBusy = false;
    }
  }
  save(as = false) {
    return this.command(() => this.write(as));
  }
  private async write(as: boolean): Promise<boolean> {
    const snapshot = this.editor.projectText();
    if (!this.platform.desktop) {
      const result = await this.platform.exportFile(
        'project',
        new Blob([snapshot], { type: 'application/json' }),
        `${filename(this.editor.doc.name)}.brickpress.json`
      );
      if (result.status === 'error') throw result.error;
      if (result.status !== 'success') return false;
      this.lastSaved = this.cleanSnapshot = snapshot;
      this.editor.dirty = this.editor.projectText() !== snapshot;
      this.editor.notify('Editable project saved.');
      return true;
    }
    let selected = !as ? this.activeFile : null;
    if (!selected) {
      const result = await this.platform.chooseSaveFile(
        'project',
        `${filename(this.editor.doc.name)}.brickpress.json`
      );
      if (result.status === 'error') throw result.error;
      if (result.status !== 'success') return false;
      selected = result.value;
    }
    const result = await this.platform.writeFile(selected, new TextEncoder().encode(snapshot));
    if (result.status !== 'success') {
      if (selected !== this.activeFile) await this.platform.releaseFile(selected);
      if (result.status === 'error' && result.error.code === 'conflict')
        return (await this.platform.confirmConflict()) ? this.write(true) : false;
      if (result.status === 'error') throw result.error;
      return false;
    }
    const previous = this.activeFile;
    this.activeFile = selected;
    this.editor.activeFilePath = selected.path;
    this.lastSaved = this.cleanSnapshot = snapshot;
    this.editor.dirty = this.editor.projectText() !== snapshot;
    // Retarget only after the replacement succeeded. A later edit remains dirty.
    if (previous && previous !== selected) await this.platform.releaseFile(previous);
    await this.platform.setDocumentTitle(this.editor.doc.name, this.editor.dirty);
    this.editor.notify(
      this.editor.dirty
        ? 'Project saved. Newer edits still need saving.'
        : 'Editable project saved.'
    );
    return true;
  }
  private async protect(message: string): Promise<UnsavedChoice | 'clean'> {
    while (this.editor.dirty) {
      const choice = await this.platform.confirmUnsaved(this.editor.doc.name, message);
      if (choice !== 'save') return choice;
      if (!(await this.write(false))) return 'cancel';
      // The user may edit while a save is in flight; protect those newer edits too.
    }
    return 'clean';
  }
  private async discardedRecovery() {
    await this.platform.writeState('recovery', this.lastSaved);
  }
  private async releaseActive() {
    if (this.activeFile) await this.platform.releaseFile(this.activeFile);
    this.activeFile = null;
    this.editor.activeFilePath = null;
    this.lastSaved = null;
  }
  newDocument() {
    return this.command(async () => {
      const choice = await this.protect(
        'Start a new document? Save the current project first if you want to keep it.'
      );
      if (choice === 'cancel') return false;
      this.closing = this.editor.transitioning = true;
      try {
        if (choice === 'discard' && this.platform.desktop) await this.discardedRecovery();
        await this.releaseActive();
        this.editor.newDocument();
        this.cleanSnapshot = this.editor.projectText();
        await this.platform.setDocumentTitle(this.editor.doc.name, false);
        return true;
      } finally {
        this.closing = this.editor.transitioning = false;
      }
    });
  }
  open() {
    return this.command(async () => {
      const result = await this.platform.chooseFile('project');
      if (result.status === 'error') throw result.error;
      return result.status === 'success' ? this.openCandidate(result.value) : false;
    });
  }
  openBrowserFile(file: File) {
    return this.command(() => this.openCandidate({ file, selected: null }));
  }
  private async openCandidate(candidate: OpenedFile): Promise<boolean> {
    let accepted = false;
    try {
      if (candidate.file.size > 20_000_000)
        throw new Error('Project files must be smaller than 20 MB.');
      const text = await candidate.file.text();
      const checked = parseDocument(text);
      const choice = await this.protect(
        'Open this project and replace the current composition? Save first if you want to keep it.'
      );
      if (choice === 'cancel') return false;
      this.closing = this.editor.transitioning = true;
      try {
        if (choice === 'discard' && this.platform.desktop) await this.discardedRecovery();
        await this.releaseActive();
        // Parsing is already complete; no failing input can clear history or tracing.
        this.editor.load(JSON.stringify(checked));
        this.activeFile = candidate.selected;
        this.editor.activeFilePath = candidate.selected?.path ?? null;
        this.lastSaved = this.cleanSnapshot = this.editor.projectText();
        accepted = true;
        await this.platform.setDocumentTitle(this.editor.doc.name, false);
        return true;
      } finally {
        this.closing = this.editor.transitioning = false;
      }
    } finally {
      if (!accepted && candidate.selected) await this.platform.releaseFile(candidate.selected);
    }
  }
  close() {
    return this.command(async () => {
      for (;;) {
        const choice = await this.protect('Save changes before closing?');
        if (choice === 'cancel') return false;
        const snapshot = this.editor.projectText();
        this.closing = this.editor.transitioning = true;
        try {
          await this.flushStorage(choice !== 'discard');
          if (choice === 'discard') await this.discardedRecovery();
          await this.platform.flush();
          if (this.editor.projectText() !== snapshot) continue;
          await this.platform.destroyWindow();
          return true;
        } finally {
          this.closing = this.editor.transitioning = false;
        }
      }
    });
  }
}
