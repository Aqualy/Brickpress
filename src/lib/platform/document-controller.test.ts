import { describe, expect, it, vi } from 'vitest';
import { createDocument, serializeDocument } from '../persistence/document';
import type { PressDocument } from '../types/document';
import { DocumentController, type DocumentEditor } from './document-controller';
import { PersistenceQueue } from './queue';
import type { PlatformAdapter, SelectedFile, UnsavedChoice } from './types';

const handle = (token = 'one', path = 'C:\\作品\\épreuve.brickpress.json'): SelectedFile => ({
  token,
  path,
  name: 'épreuve.brickpress.json',
  mime: 'application/json'
});
function candidate(text: string) {
  return { size: text.length, text: async () => text } as File;
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
function setup() {
  const editor: DocumentEditor & {
    doc: PressDocument;
    history: string[];
    trace: string | null;
    selected: string[];
  } = {
    doc: createDocument(),
    dirty: false,
    documentBusy: false,
    transitioning: false,
    activeFilePath: null,
    history: ['undo'],
    trace: 'guide',
    selected: ['one'],
    projectText() {
      return serializeDocument(this.doc);
    },
    load(text, recovered = false) {
      this.doc = JSON.parse(text);
      this.dirty = recovered;
      this.history = [];
      this.trace = null;
      this.selected = [];
    },
    newDocument() {
      this.doc = createDocument();
      this.dirty = false;
      this.history = [];
      this.trace = null;
      this.selected = [];
    },
    notify: vi.fn()
  };
  const platform: PlatformAdapter = {
    desktop: true,
    chooseFile: vi.fn(async () => ({ status: 'cancelled' }) as const),
    chooseSaveFile: vi.fn(async () => ({ status: 'success', value: handle() }) as const),
    writeFile: vi.fn(async () => ({ status: 'success', value: undefined }) as const),
    releaseFile: vi.fn(async () => {}),
    exportFile: vi.fn(async () => ({ status: 'success', value: undefined }) as const),
    readState: vi.fn(async () => null),
    writeState: vi.fn(async () => {}),
    quarantineState: vi.fn(async () => {}),
    loadTrace: vi.fn(async () => null),
    saveTrace: vi.fn(async () => {}),
    confirmAction: vi.fn(async () => true),
    confirmUnsaved: vi.fn(async () => 'cancel' as UnsavedChoice),
    confirmConflict: vi.fn(async () => false),
    setDocumentTitle: vi.fn(async () => {}),
    onCloseRequested: vi.fn(async () => () => {}),
    destroyWindow: vi.fn(async () => {}),
    flush: vi.fn(async () => {})
  };
  const flush = vi.fn(async (_includeRecovery: boolean) => {});
  const controller = new DocumentController(editor, platform, flush);
  const edit = (name = 'Changed') => {
    editor.doc = { ...editor.doc, name };
    editor.dirty = true;
  };
  return { editor, platform, controller, flush, edit };
}

describe('native document controller', () => {
  it('saves untitled documents through a picker, then writes the active Unicode path', async () => {
    const { editor, platform, controller, edit } = setup();
    edit();
    expect(await controller.save()).toBe(true);
    expect(editor.activeFilePath).toBe(handle().path);
    expect(editor.dirty).toBe(false);
    const saved = controller.lastSaved;
    edit('Second');
    expect(await controller.save()).toBe(true);
    expect(platform.chooseSaveFile).toHaveBeenCalledTimes(1);
    expect(new TextDecoder().decode(vi.mocked(platform.writeFile).mock.calls[0][1])).toBe(saved);
    expect(new TextDecoder().decode(vi.mocked(platform.writeFile).mock.calls[1][1])).toContain(
      'Second'
    );
  });
  it('captures a snapshot and leaves edits made during a write unsaved', async () => {
    const { editor, platform, controller, edit } = setup();
    const write = deferred<{ status: 'success'; value: undefined }>();
    vi.mocked(platform.writeFile).mockReturnValueOnce(write.promise);
    edit('Captured');
    const pending = controller.save();
    await vi.waitFor(() => expect(platform.writeFile).toHaveBeenCalled());
    edit('After capture');
    write.resolve({ status: 'success', value: undefined });
    expect(await pending).toBe(true);
    expect(controller.lastSaved).toContain('Captured');
    expect(controller.lastSaved).not.toContain('After capture');
    expect(editor.dirty).toBe(true);
  });
  it('cancellation preserves document, history, trace and selection', async () => {
    const { editor, platform, controller, edit } = setup();
    edit();
    const doc = editor.doc;
    vi.mocked(platform.chooseSaveFile).mockResolvedValueOnce({ status: 'cancelled' });
    expect(await controller.save()).toBe(false);
    expect(editor.doc).toBe(doc);
    expect(editor).toMatchObject({
      dirty: true,
      trace: 'guide',
      history: ['undo'],
      selected: ['one']
    });
    expect(controller.activeFile).toBeNull();
    expect(platform.writeFile).not.toHaveBeenCalled();
  });
  it('failed Save As releases the candidate and preserves the active file and saved snapshot', async () => {
    const { editor, platform, controller, edit } = setup();
    await controller.save();
    const saved = controller.lastSaved,
      active = controller.activeFile;
    edit();
    vi.mocked(platform.chooseSaveFile).mockResolvedValueOnce({
      status: 'success',
      value: handle('two')
    });
    vi.mocked(platform.writeFile).mockResolvedValueOnce({
      status: 'error',
      error: { code: 'io', message: 'Disk full' }
    });
    expect(await controller.save(true)).toBe(false);
    expect(controller.activeFile).toBe(active);
    expect(controller.lastSaved).toBe(saved);
    expect(editor.dirty).toBe(true);
    expect(platform.releaseFile).toHaveBeenLastCalledWith(handle('two'));
    expect(editor.notify).toHaveBeenLastCalledWith('Disk full');
  });
  it('Save As retargets only after a successful replacement', async () => {
    const { platform, controller, edit } = setup();
    await controller.save();
    edit();
    vi.mocked(platform.chooseSaveFile).mockResolvedValueOnce({
      status: 'success',
      value: handle('two', '/Users/版画/second.brickpress.json')
    });
    expect(await controller.save(true)).toBe(true);
    expect(controller.activeFile?.token).toBe('two');
    expect(platform.releaseFile).toHaveBeenCalledWith(handle());
    expect(platform.chooseSaveFile).toHaveBeenCalledTimes(2);
  });
  it('external changes offer Save As; cancellation preserves dirty state and the existing handle', async () => {
    const { editor, platform, controller, edit } = setup();
    await controller.save();
    edit();
    vi.mocked(platform.writeFile).mockResolvedValueOnce({
      status: 'error',
      error: { code: 'conflict', message: 'Changed externally' }
    });
    expect(await controller.save()).toBe(false);
    expect(platform.confirmConflict).toHaveBeenCalledOnce();
    expect(controller.activeFile?.token).toBe('one');
    expect(editor.dirty).toBe(true);
    expect(platform.chooseSaveFile).toHaveBeenCalledTimes(1);
  });
  it('external-change Save As can successfully preserve both copies', async () => {
    const { platform, controller, edit } = setup();
    await controller.save();
    edit();
    vi.mocked(platform.writeFile).mockResolvedValueOnce({
      status: 'error',
      error: { code: 'conflict', message: 'Changed externally' }
    });
    vi.mocked(platform.confirmConflict).mockResolvedValueOnce(true);
    vi.mocked(platform.chooseSaveFile).mockResolvedValueOnce({
      status: 'success',
      value: handle('copy')
    });
    expect(await controller.save()).toBe(true);
    expect(controller.activeFile?.token).toBe('copy');
  });
  it('new-document Save cancellation or failure protects the current composition', async () => {
    const { editor, platform, controller, edit } = setup();
    edit();
    const before = editor.projectText();
    vi.mocked(platform.confirmUnsaved).mockResolvedValue('save');
    vi.mocked(platform.chooseSaveFile).mockResolvedValueOnce({ status: 'cancelled' });
    expect(await controller.newDocument()).toBe(false);
    expect(editor.projectText()).toBe(before);
    vi.mocked(platform.writeFile).mockResolvedValueOnce({
      status: 'error',
      error: { code: 'io', message: 'Cannot write' }
    });
    expect(await controller.newDocument()).toBe(false);
    expect(editor.projectText()).toBe(before);
    expect(editor.history).toEqual(['undo']);
  });
  it('recovered projects are unsaved and have no file handle even after restarting', async () => {
    const { editor, platform, controller } = setup();
    controller.recover(serializeDocument(createDocument(true)));
    expect(editor.dirty).toBe(true);
    expect(controller.lastSaved).toBeNull();
    expect(controller.activeFile).toBeNull();
    expect(await controller.save()).toBe(true);
    expect(platform.chooseSaveFile).toHaveBeenCalledOnce();
  });
  it('invalid Open input does not clear history, selection or the tracing image', async () => {
    const { editor, platform, controller, edit } = setup();
    edit();
    const before = editor.doc;
    vi.mocked(platform.chooseFile).mockResolvedValueOnce({
      status: 'success',
      value: { file: candidate('{"version":99}'), selected: handle('invalid') }
    });
    expect(await controller.open()).toBe(false);
    expect(editor.doc).toBe(before);
    expect(editor).toMatchObject({ trace: 'guide', selected: ['one'], history: ['undo'] });
    expect(platform.confirmUnsaved).not.toHaveBeenCalled();
    expect(platform.releaseFile).toHaveBeenCalledWith(handle('invalid'));
  });
  it('cancelled Open protection releases the candidate without changing the current active file', async () => {
    const { editor, platform, controller, edit } = setup();
    await controller.save();
    edit();
    vi.mocked(platform.chooseFile).mockResolvedValueOnce({
      status: 'success',
      value: {
        file: candidate(serializeDocument(createDocument(true))),
        selected: handle('candidate')
      }
    });
    expect(await controller.open()).toBe(false);
    expect(controller.activeFile?.token).toBe('one');
    expect(editor.history).toEqual(['undo']);
    expect(platform.releaseFile).toHaveBeenCalledWith(handle('candidate'));
  });
  it('opens legacy version-one data without putting native paths into PressDocument', async () => {
    const { editor, platform, controller } = setup();
    vi.mocked(platform.chooseFile).mockResolvedValueOnce({
      status: 'success',
      value: {
        file: candidate(serializeDocument(createDocument(true))),
        selected: { ...handle('legacy'), name: 'legacy.legopress.json' }
      }
    });
    expect(await controller.open()).toBe(true);
    expect(editor.doc.version).toBe(1);
    expect(editor.projectText()).not.toContain('C:\\');
    expect(editor.activeFilePath).toBe(handle().path);
    expect(editor.dirty).toBe(false);
  });
  it('discard restores recovery to the last saved snapshot before closing', async () => {
    const { platform, controller, flush, edit } = setup();
    await controller.save();
    const saved = controller.lastSaved;
    edit();
    vi.mocked(platform.confirmUnsaved).mockResolvedValueOnce('discard');
    expect(await controller.close()).toBe(true);
    expect(flush).toHaveBeenCalledWith(false);
    expect(platform.writeState).toHaveBeenCalledWith('recovery', saved);
    expect(platform.destroyWindow).toHaveBeenCalledOnce();
  });
  it('discard clears recovery for a never-saved document', async () => {
    const { platform, controller, edit } = setup();
    edit();
    vi.mocked(platform.confirmUnsaved).mockResolvedValueOnce('discard');
    expect(await controller.close()).toBe(true);
    expect(platform.writeState).toHaveBeenCalledWith('recovery', null);
  });
  it('cancelled close never flushes or destroys the window', async () => {
    const { platform, controller, flush, edit } = setup();
    edit();
    expect(await controller.close()).toBe(false);
    expect(flush).not.toHaveBeenCalled();
    expect(platform.destroyWindow).not.toHaveBeenCalled();
  });
  it('a failed storage flush keeps the window and document open', async () => {
    const { editor, platform, controller, flush } = setup();
    flush.mockRejectedValueOnce(new Error('Preferences could not be written'));
    expect(await controller.close()).toBe(false);
    expect(platform.destroyWindow).not.toHaveBeenCalled();
    expect(editor.notify).toHaveBeenLastCalledWith('Preferences could not be written');
    expect(controller.closing).toBe(false);
    expect(editor.documentBusy).toBe(false);
  });
  it('protects edits made while close-time persistence is flushing', async () => {
    const { platform, controller, flush, edit } = setup();
    flush.mockImplementationOnce(async () => {
      edit('Edited while flushing');
    });
    expect(await controller.close()).toBe(false);
    expect(platform.confirmUnsaved).toHaveBeenCalledOnce();
    expect(platform.destroyWindow).not.toHaveBeenCalled();
  });
  it('serializes document commands while still allowing artwork edits during saves', async () => {
    const { platform, controller, edit } = setup();
    const write = deferred<{ status: 'success'; value: undefined }>();
    vi.mocked(platform.writeFile).mockReturnValueOnce(write.promise);
    const pending = controller.save();
    expect(await controller.newDocument()).toBe(false);
    edit();
    write.resolve({ status: 'success', value: undefined });
    expect(await pending).toBe(true);
  });
  it('browser Save still downloads and avoids native pickers', async () => {
    const { editor, platform, controller, edit } = setup();
    platform.desktop = false;
    edit();
    expect(await controller.save()).toBe(true);
    expect(platform.exportFile).toHaveBeenCalledOnce();
    expect(platform.chooseSaveFile).not.toHaveBeenCalled();
    expect(editor.dirty).toBe(false);
  });
});

describe('serialized persistence', () => {
  it('preserves write order after failure and blocks flush until the failed record is repaired', async () => {
    const queue = new PersistenceQueue(),
      calls: string[] = [];
    await expect(
      queue.run('recovery', async () => {
        calls.push('failed');
        throw new Error('Disk full');
      })
    ).rejects.toThrow('Disk full');
    await queue.run('presets', async () => {
      calls.push('presets');
    });
    await expect(queue.flush()).rejects.toThrow('Disk full');
    await queue.run('recovery', async () => {
      calls.push('repaired');
    });
    await queue.flush();
    expect(calls).toEqual(['failed', 'presets', 'repaired']);
  });
});
