import { browser, $, $$ } from '@wdio/globals';
import type {} from 'webdriverio';
import type {} from '@wdio/tauri-service';
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { createDocument, serializeDocument } from '../../src/lib/persistence/document';
import type { PressDocument } from '../../src/lib/types/document';

async function invoke<T = unknown>(
  command: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  return browser.tauri.execute(
    ({ core }, command, args) => core.invoke(command, args),
    command,
    args
  ) as Promise<T>;
}
async function ready() {
  await browser.waitUntil(async () => (await $('.editor-app').getAttribute('inert')) === null);
  const hint = await $('[aria-label="Dismiss editor hint"]');
  if (await hint.isDisplayed()) await hint.click();
}
async function reload() {
  const origin = await browser.execute(() => performance.timeOrigin);
  await browser.refresh();
  await browser.waitUntil(
    async () => (await browser.execute(() => performance.timeOrigin)) !== origin
  );
  await ready();
}
async function reset(doc = createDocument()) {
  await invoke('test_reset');
  await invoke('write_state', { key: 'recovery', value: serializeDocument(doc) });
  await reload();
}
async function button(name: string) {
  return $(`button*=${name}`);
}
async function title() {
  return invoke<string>('plugin:window|title', { label: 'main' });
}
async function menu() {
  await $('[aria-label="Main menu"]').click();
}
async function pieceCount(count: number) {
  await browser.waitUntil(async () => (await countSelector('.artboard [data-uid]')) === count);
}
async function countSelector(selector: string) {
  return browser.execute((selector) => document.querySelectorAll(selector).length, selector);
}
async function recovery(): Promise<PressDocument> {
  await browser.waitUntil(async () =>
    (await $('.app-footer .local-save').getText()).includes('Saved on this device')
  );
  let raw = '';
  await browser.waitUntil(async () => {
    raw = await invoke<string>('read_state', { key: 'recovery' });
    return Boolean(raw);
  });
  return JSON.parse(raw);
}
async function chosen(name: string, data = '') {
  const path = await invoke<string>('test_fixture', { name, data });
  await invoke('test_queue_file', { path });
  return path;
}
async function choice(choice: string) {
  await invoke('test_queue_choice', { choice });
}
async function dialogsHandled() {
  await browser.waitUntil(async () =>
    (await invoke<number[]>('test_pending_dialogs')).every((count) => count === 0)
  );
  await browser.waitUntil(async () => !(await $('button*=Save').getAttribute('disabled')));
}
async function name(value: string) {
  await menu();
  await browser.execute((value) => {
    const input = document.querySelector<HTMLInputElement>('[aria-label="Document name"]')!;
    input.value = value;
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }, value);
  await browser.keys('Escape');
  await browser.waitUntil(async () => (await title()).includes(value));
}
const commandKey = process.platform === 'darwin' ? 'Meta' : 'Control';
async function shortcut(key: string) {
  await browser.keys([commandKey, key]);
}

describe('Brickpress native webview', () => {
  before(async () => {
    await browser.switchToWindow('main');
    await ready();
  });
  beforeEach(async () => {
    await reset();
  });

  it('saves Unicode paths, updates the same file, and Save As retargets', async () => {
    await name('印刷 café');
    await chosen('印刷 café.brickpress.json');
    await button('Save').then((b) => b.click());
    await browser.waitUntil(async () =>
      (await invoke<string>('test_read_file', { name: '印刷 café.brickpress.json' })).includes(
        '印刷 café'
      )
    );
    await name('Second revision');
    await button('Save').then((b) => b.click());
    await browser.waitUntil(async () =>
      (await invoke<string>('test_read_file', { name: '印刷 café.brickpress.json' })).includes(
        'Second revision'
      )
    );
    await chosen('copy.brickpress.json');
    await menu();
    await button('Save As').then((b) => b.click());
    await browser.waitUntil(async () =>
      (await invoke<string>('test_read_file', { name: 'copy.brickpress.json' })).includes(
        'Second revision'
      )
    );
    assert.ok((await title()).includes('Second revision'));
  });

  it('keeps edits after cancelled Save and rejects invalid Open without replacing artwork', async () => {
    await name('Unsaved work');
    await invoke('test_queue_file', { path: null });
    await button('Save').then((b) => b.click());
    assert.ok((await title()).includes('●'));
    await chosen('invalid.legopress.json', '{"version":999}');
    await button('Open').then((b) => b.click());
    await browser.waitUntil(async () =>
      (await $('[role="status"]').getText()).includes('supported')
    );
    assert.equal((await recovery()).name, 'Unsaved work');
    assert.ok((await title()).includes('Unsaved work'));
  });

  it('detects an external edit and offers Save As while keeping the outside file', async () => {
    await chosen('outside.brickpress.json');
    await button('Save').then((b) => b.click());
    await browser.waitUntil(async () => !(await title()).includes('●'));
    await invoke('test_corrupt_file', {
      name: 'outside.brickpress.json',
      data: 'outside revision'
    });
    await name('My revision');
    await choice('ok');
    await chosen('preserved.brickpress.json');
    await button('Save').then((b) => b.click());
    await browser.waitUntil(async () =>
      (await invoke<string>('test_read_file', { name: 'preserved.brickpress.json' })).includes(
        'My revision'
      )
    );
    assert.equal(
      await invoke('test_read_file', { name: 'outside.brickpress.json' }),
      'outside revision'
    );
  });

  it('protects New, persists recovery, and keeps browser storage separate', async () => {
    await name('Keep me');
    await choice('cancel');
    await button('New').then((b) => b.click());
    assert.ok((await title()).includes('Keep me'));
    await recovery();
    await reload();
    assert.ok((await title()).includes('Keep me'));
    assert.ok((await title()).includes('●'));
    await choice('discard');
    await button('New').then((b) => b.click());
    await browser.waitUntil(async () => (await title()).includes('Untitled impression'));
    assert.equal(
      await browser.execute(() => localStorage.getItem('form-impression:document:v1')),
      null
    );
  });

  it('cancels a native window close and a cancelled save during close', async () => {
    await name('Close protection');
    await choice('cancel');
    await invoke('test_request_close');
    await dialogsHandled();
    await ready();
    assert.ok((await title()).includes('Close protection'));
    await choice('save');
    await invoke('test_queue_file', { path: null });
    await invoke('test_request_close');
    await dialogsHandled();
    assert.ok((await title()).includes('●'));
    assert.ok(await $('.artboard').isDisplayed());
  });

  it('places using the keyboard and samples with middle click in the native SVG editor', async () => {
    await $('.piece-card').click();
    await browser.keys(['ArrowRight', 'ArrowDown', 'r', 'Enter']);
    await pieceCount(1);
    const target = await $('.artboard [data-uid]');
    await target.click({ button: 'middle' });
    assert.equal(await $('[aria-label="Place tool"]').getAttribute('aria-pressed'), 'true');
    await browser.keys(['ArrowRight', 'ArrowRight', 'Enter']);
    await pieceCount(2);
    await button('Design').then((b) => b.click());
    await button('Print preview').then((b) => b.click());
    assert.ok((await countSelector('.artboard filter')) > 0);
  });

  it('routes application Quit through document protection', async () => {
    await name('Quit protection');
    await choice('cancel');
    await invoke('test_request_quit');
    await dialogsHandled();
    await browser.waitUntil(async () =>
      (await $('.app-footer .local-save').getText()).includes('Saved on this device')
    );
    assert.ok((await title()).includes('Quit protection'));
    assert.ok((await title()).includes('●'));
    await choice('save');
    await invoke('test_queue_file', { path: null });
    await invoke('test_request_quit');
    await dialogsHandled();
    assert.ok((await title()).includes('●'));
    assert.ok(await $('.artboard').isDisplayed());
  });

  it('keeps tab arrows scoped, canvas zoom working, and the embossed view separate', async () => {
    const before = JSON.stringify(await recovery());
    await browser.execute(() =>
      [...document.querySelectorAll<HTMLElement>('[role="tab"]')]
        .find((node) => node.textContent?.trim() === 'Properties')
        ?.focus()
    );
    await browser.keys('ArrowRight');
    await browser.waitUntil(
      async () =>
        (await browser.execute(() => document.activeElement?.textContent?.trim())) === 'Layers'
    );
    assert.equal(
      await browser.execute(() => document.activeElement?.textContent?.trim()),
      'Layers'
    );
    await browser.keys('ArrowRight');
    await browser.waitUntil(
      async () =>
        (await browser.execute(() => document.activeElement?.textContent?.trim())) === 'Export'
    );
    assert.equal(
      await browser.execute(() => document.activeElement?.textContent?.trim()),
      'Export'
    );
    const beforeZoom = parseInt(await $('.zoom-value').getText());
    await $('[aria-label="Canvas zoom in"]').click();
    const zoom = await browser.execute(() =>
      [...document.querySelectorAll('.zoom-value')].map((node) => node.textContent)
    );
    assert.equal(zoom.length, 1);
    assert.ok(parseInt(zoom[0]!) > beforeZoom);
    await $('[aria-label="Grid settings"]').click();
    // The embedded driver's programmatic option.click() does not select HTML
    // options. Send the same value/change event produced by the native control.
    await browser.execute(() => {
      const select = document.querySelector<HTMLSelectElement>('[aria-label="Grid appearance"]')!;
      select.value = 'embossed';
      select.dispatchEvent(new Event('change', { bubbles: true }));
    });
    await browser.waitUntil(
      async () => (await countSelector('[data-editor-guide="embossed"]')) === 1
    );
    await browser.keys('Escape');
    assert.equal(await countSelector('[data-editor-guide="embossed"]'), 1);
    assert.equal(JSON.stringify(await recovery()), before);
    await button('Print preview').then((b) => b.click());
    assert.equal(await countSelector('[data-editor-guide="embossed"]'), 0);
  });

  it('exports transparent PNG/SVG and a project copy without clearing unsaved status', async () => {
    await $('.piece-card').click();
    await browser.keys('Enter');
    await pieceCount(1);
    await $('[role="tab"][data-value="export"]').click();
    await $('[aria-label="Include paper background"]').click();
    await chosen('artwork.png');
    await button('Export PNG').then((b) => b.click());
    await browser.waitUntil(async () =>
      (await $('[role="status"]').getText()).includes('Export ready')
    );
    const png = await readFile(resolve('artifacts/native/data/test-files/artwork.png'));
    assert.deepEqual([...png.subarray(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10]);
    const alpha = await browser.execute(async (base64) => {
      const image = new Image();
      image.src = `data:image/png;base64,${base64}`;
      await image.decode();
      const canvas = document.createElement('canvas');
      canvas.width = image.width;
      canvas.height = image.height;
      const context = canvas.getContext('2d')!;
      context.drawImage(image, 0, 0);
      return context.getImageData(image.width - 1, image.height - 1, 1, 1).data[3];
    }, png.toString('base64'));
    assert.equal(alpha, 0);
    await button('SVG').then((b) => b.click());
    await chosen('artwork.svg');
    await button('Export SVG').then((b) => b.click());
    await browser.waitUntil(async () =>
      (await invoke<string>('test_read_file', { name: 'artwork.svg' })).startsWith('<svg')
    );
    const svg = await invoke<string>('test_read_file', { name: 'artwork.svg' });
    assert.ok(!svg.includes('trace-overlay'));
    assert.ok(!svg.includes('data-paper'));
    await button('Project').then((b) => b.click());
    await chosen('export.brickpress.json');
    await button('Export project').then((b) => b.click());
    await browser.waitUntil(async () =>
      (await invoke<string>('test_read_file', { name: 'export.brickpress.json' })).includes(
        'version'
      )
    );
    assert.ok((await title()).includes('●'));
  });

  it('previews exports and recovers custom print presets in native app storage', async () => {
    assert.equal(await countSelector('[aria-label="Pressure"]'), 0);
    await button('Print preview').then((b) => b.click());
    await browser.execute(() => {
      const slider = document.querySelector<HTMLInputElement>('[aria-label="Pressure"]')!;
      slider.value = '.23';
      slider.dispatchEvent(new Event('change', { bubbles: true }));
    });
    await $('[aria-label="Print preset name"]').setValue('Native cotton');
    await $('.preset-save-row button').click();
    await browser.waitUntil(
      async () =>
        JSON.parse(await invoke<string>('read_state', { key: 'preferences' })).printPresets
          .length === 1
    );
    const settings = JSON.parse(await invoke<string>('read_state', { key: 'preferences' }));
    assert.equal(settings.printPresets[0].settings.pressure, 0.23);
    await reload();
    await button('Print preview').then((b) => b.click());
    await browser.execute(() => {
      const select = document.querySelector<HTMLSelectElement>('[aria-label="Print preset"]')!;
      select.value = select.querySelector<HTMLOptionElement>(
        'optgroup[label="Your presets"] option'
      )!.value;
      select.dispatchEvent(new Event('change', { bubbles: true }));
    });
    assert.equal(await $('[aria-label="Pressure"]').getValue(), '0.23');
    await $('[role="tab"][data-value="export"]').click();
    await browser.waitUntil(async () =>
      browser.execute(() => {
        const image = document.querySelector<HTMLImageElement>('.export-preview img');
        return !!image && image.complete && image.naturalWidth > 0;
      })
    );
    await browser.saveScreenshot('artifacts/native/export-preview.png');
  });

  it('keeps pass keyboard focus after reordering', async () => {
    await reset(createDocument(true));
    await $('[role="tab"][data-value="layers"]').click();
    await browser.execute(() =>
      document
        .querySelector<HTMLButtonElement>(
          '[role="tabpanel"][data-state="active"] [aria-label="Reorder 01 · Vermilion"]'
        )!
        .focus()
    );
    await browser.keys('ArrowDown');
    await browser.waitUntil(
      async () =>
        (await browser.execute(() => document.activeElement?.getAttribute('aria-label'))) ===
        'Reorder 01 · Vermilion'
    );
    const rows = await $$('[role="tabpanel"][data-state="active"] .pass-block');
    assert.equal(await rows[1].$('.pass-name').getValue(), '01 · Vermilion');
    await browser.saveScreenshot('artifacts/native/pass-reordering.png');
  });

  it('uploads a tracing image through binary IPC and excludes it from the project', async () => {
    const directory = resolve('artifacts/native/data/test-files');
    await mkdir(directory, { recursive: true });
    const path = resolve(directory, 'guide.png');
    const base64 = await browser.execute(() => {
      const canvas = document.createElement('canvas');
      canvas.width = 160;
      canvas.height = 80;
      const context = canvas.getContext('2d')!;
      context.fillStyle = '#005ac5';
      context.fillRect(0, 0, 160, 80);
      return canvas.toDataURL('image/png').split(',')[1];
    });
    await writeFile(path, Buffer.from(base64, 'base64'));
    await invoke('test_queue_file', { path });
    await $('[aria-label="Grid settings"]').click();
    await button('Add tracing image').then((b) => b.click());
    await browser.waitUntil(
      async () => (await countSelector('[data-editor-guide="tracing"]')) === 1
    );
    await browser.keys('Escape');
    await browser.waitUntil(async () =>
      Boolean(await invoke('read_state', { key: 'trace-metadata' }))
    );
    await reload();
    assert.equal(await countSelector('[data-editor-guide="tracing"]'), 1);
    const project = JSON.stringify(await recovery());
    assert.ok(!project.includes('assetId'));
    assert.ok(!project.includes('guide.png'));
  });

  it('checks accessibility in both modes and every inspector tab', async () => {
    const axe = await readFile(resolve('node_modules/axe-core/axe.min.js'), 'utf8');
    await browser.execute(axe);
    for (const mode of ['Design', 'Print preview']) {
      await button(mode).then((b) => b.click());
      for (const tab of ['Properties', 'Layers', 'Export']) {
        await $(`[role="tab"]=${tab}`).click();
        const violations = await browser.execute(async () => {
          const axe = (
            window as unknown as {
              axe: { run: (options: unknown) => Promise<{ violations: unknown[] }> };
            }
          ).axe;
          return (
            await axe.run({
              runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'] }
            })
          ).violations;
        });
        assert.deepEqual(violations, []);
      }
    }
  });

  it('decodes every supported tracing format and the image-element fallback', async () => {
    const directory = resolve('artifacts/native/data/test-files');
    await mkdir(directory, { recursive: true });
    const extensions = ['png', 'jpg', 'webp', 'gif', 'avif', 'bmp'];
    for (const [index, extension] of extensions.entries()) {
      await $('[aria-label="Grid settings"]').click();
      const name = `decode-${index}.${extension}`;
      const path = resolve(directory, name);
      await writeFile(path, await readFile(resolve('tests/native/fixtures', `guide.${extension}`)));
      await invoke('test_queue_file', { path });
      await button(index ? 'Replace image' : 'Add tracing image').then((element) =>
        element.click()
      );
      await dialogsHandled();
      await browser.waitUntil(
        async () => {
          const metadata = await invoke<string | null>('read_state', { key: 'trace-metadata' });
          return metadata !== null && JSON.parse(metadata).name === name;
        },
        { timeoutMsg: `Tracing import did not persist ${name}` }
      );
      assert.equal(await countSelector('[data-editor-guide="tracing"]'), 1);
      await browser.waitUntil(async () =>
        browser.execute(() => {
          const img = document.querySelector<HTMLImageElement>('.trace-summary img');
          return !!img && img.complete && img.naturalWidth > 0;
        })
      );
      await browser.keys('Escape');
    }
    await browser.execute(() => {
      Object.defineProperty(window, 'createImageBitmap', { value: undefined, configurable: true });
    });
    const path = resolve(directory, 'fallback.png');
    await writeFile(path, await readFile(resolve('tests/native/fixtures/guide.png')));
    await invoke('test_queue_file', { path });
    await $('[aria-label="Grid settings"]').click();
    await button('Replace image').then((element) => element.click());
    await dialogsHandled();
    await browser.waitUntil(async () => {
      const metadata = await invoke<string | null>('read_state', { key: 'trace-metadata' });
      return metadata !== null && JSON.parse(metadata).name === 'fallback.png';
    });
    await browser.keys('Escape');
    await browser.execute(() => {
      const NativeImage = window.Image;
      const create = URL.createObjectURL.bind(URL);
      const avifUrls = new Set<string>();
      URL.createObjectURL = (blob) => {
        const url = create(blob);
        if (blob instanceof Blob && blob.type === 'image/avif') avifUrls.add(url);
        return url;
      };
      window.Image = new Proxy(NativeImage, {
        construct(target, args) {
          const image = Reflect.construct(target, args) as HTMLImageElement;
          const src = Object.getOwnPropertyDescriptor(HTMLImageElement.prototype, 'src')!;
          Object.defineProperty(image, 'src', {
            set(value: string) {
              if (avifUrls.has(value))
                queueMicrotask(() => image.dispatchEvent(new Event('error')));
              else src.set!.call(image, value);
            },
            get() {
              return src.get!.call(image);
            }
          });
          return image;
        }
      });
    });
    const avif = resolve(directory, 'fallback.avif');
    await writeFile(avif, await readFile(resolve('tests/native/fixtures/guide.avif')));
    await invoke('test_queue_file', { path: avif });
    await $('[aria-label="Grid settings"]').click();
    await button('Replace image').then((element) => element.click());
    await dialogsHandled();
    await browser.waitUntil(async () => {
      const metadata = await invoke<string | null>('read_state', { key: 'trace-metadata' });
      return metadata !== null && JSON.parse(metadata).name === 'fallback.avif';
    });
    await reload();
    await $('[aria-label="Grid settings"]').click();
    await browser.waitUntil(async () =>
      browser.execute(() => {
        const img = document.querySelector<HTMLImageElement>('.trace-summary img');
        return !!img && img.complete && img.naturalWidth > 0;
      })
    );
    await browser.keys('Escape');
  });

  it('reads bundled JavaScript and Rust notices and restores focus after dismissal', async () => {
    await menu();
    await button('Third-party notices').then((element) => element.click());
    const text = await $('[aria-label="Dependency license text"]');
    await browser.waitUntil(async () => (await text.getValue()).length > 250000);
    const notices = await text.getValue();
    assert.ok(notices.includes('@tauri-apps/api'));
    assert.match(notices, /tauri.*2\.12\.1/);
    await browser.keys('Escape');
    assert.equal(
      await browser.execute(() => document.activeElement?.getAttribute('aria-label')),
      'Main menu'
    );
  });

  it('keeps the editor usable at 320px with enlarged text', async () => {
    await invoke('plugin:window|set_size', {
      label: 'main',
      value: { Logical: { width: 320, height: 640 } }
    });
    await browser.waitUntil(async () => (await browser.execute(() => innerWidth)) <= 340);
    await $('[aria-label="Open Pieces"]').click();
    assert.ok(await $('.piece-card').isDisplayed());
    await browser.keys('Escape');
    await $('[aria-label="Open Inspector"]').click();
    await browser.execute(() => {
      const selector = 'button,input,select,summary,h2,h3,h4,p,span,label';
      const sizes = [...document.querySelectorAll<HTMLElement>(selector)].map((element) => ({
        element,
        size: parseFloat(getComputedStyle(element).fontSize) * 2
      }));
      for (const { element, size } of sizes) element.style.fontSize = `${size}px`;
      const style = document.createElement('style');
      style.id = 'native-text-spacing';
      style.textContent = `${selector}{line-height:1.5!important;letter-spacing:.12em!important;word-spacing:.16em!important}`;
      document.head.append(style);
    });
    assert.ok(await $('[aria-label="Close Inspector"]').isDisplayed());
    assert.equal(
      await browser.execute(() => document.documentElement.scrollWidth > innerWidth),
      false
    );
    const clippedTabs = await browser.execute(() =>
      [...document.querySelectorAll<HTMLElement>('.inspector-tabs button')]
        .filter(
          (element) =>
            element.scrollHeight > element.clientHeight + 1 ||
            element.scrollWidth > element.clientWidth + 1
        )
        .map((element) => element.textContent?.trim())
    );
    assert.deepEqual(clippedTabs, [], 'Enlarged inspector tabs must remain readable');
    await browser.saveScreenshot('artifacts/native/320-inspector-enlarged-text.png');
    await browser.keys('Escape');
    const clippedControls = await browser.execute(() =>
      [...document.querySelectorAll<HTMLElement>('.toolbar-button,.mode-switch button')]
        .filter((element) => element.getBoundingClientRect().width > 0)
        .filter(
          (element) =>
            element.scrollHeight > element.clientHeight + 1 ||
            element.scrollWidth > element.clientWidth + 1
        )
        .map((element) => element.getAttribute('aria-label') || element.textContent?.trim())
    );
    assert.deepEqual(clippedControls, [], 'Enlarged toolbar labels must fit their controls');
    const clippedFooter = await browser.execute(() =>
      [...document.querySelectorAll<HTMLElement>('.app-footer span')]
        .filter((element) => element.getBoundingClientRect().width > 0)
        .filter((element) => {
          const range = document.createRange();
          range.selectNodeContents(element);
          const text = range.getBoundingClientRect();
          const box = element.getBoundingClientRect();
          return (
            text.left < box.left - 1 || text.right > box.right + 1 || text.bottom > box.bottom + 1
          );
        })
        .map((element) => element.textContent?.trim())
    );
    assert.deepEqual(clippedFooter, [], 'Enlarged footer text must remain readable');
    await browser.saveScreenshot('artifacts/native/320-enlarged-text.png');
    await reload();
    await invoke('plugin:window|set_size', {
      label: 'main',
      value: { Logical: { width: 1448, height: 960 } }
    });
  });

  it('renders 2,000 pieces and their print filters', async () => {
    const doc = createDocument();
    doc.board = { width: 64, height: 64 };
    doc.passes[0].pieces = Array.from({ length: 2000 }, (_, i) => ({
      uid: `native-${i}`,
      pieceId: '3070',
      x: i % 50,
      y: Math.floor(i / 50),
      rotation: 0,
      seed: i + 1
    }));
    await reset(doc);
    await pieceCount(2000);
    await button('Print preview').then((b) => b.click());
    await browser.waitUntil(async () => (await countSelector('.artboard filter')) >= 2000);
    await browser.saveScreenshot('artifacts/native/2000-pieces.png');
  });

  (process.platform === 'win32' && process.env.BRICKPRESS_REAL_DIALOG_TESTS === '1' ? it : it.skip)(
    'checks real Windows Open, Save, and close confirmation cancellation',
    async () => {
      const cancelDialog = (dialogTitle: string) =>
        promisify(execFile)(
          'pwsh.exe',
          [
            '-NoProfile',
            '-STA',
            '-File',
            resolve('tests/native/windows-dialog.ps1'),
            '-DialogTitle',
            dialogTitle,
            '-ApplicationPath',
            resolve('src-tauri/target/debug/brickpress.exe')
          ],
          { windowsHide: true, timeout: 40000 }
        );
      for (const save of [false, true]) {
        const dismissed = cancelDialog(`${save ? 'Save' : 'Open'} — Brickpress`);
        await button(save ? 'Save' : 'Open').then((element) => element.click());
        const result = await dismissed;
        await browser.waitUntil(
          async () => (await $('button*=Save').getAttribute('disabled')) === null
        );
        assert.ok(result.stdout.includes('Observed and cancelled'));
      }
      await name('Native dialog protection');
      const dismissed = cancelDialog('Unsaved document — Brickpress');
      await invoke('test_request_close');
      await dismissed;
      assert.ok((await title()).includes('Native dialog protection'));
      assert.ok(await $('.artboard').isDisplayed());
    }
  );
});
