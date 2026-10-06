import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { readFile } from 'node:fs/promises';
import { createDocument, AUTOSAVE_KEY, makePass } from '../src/lib/persistence/document';
import { pieces } from '../src/lib/catalog/catalog';
import type { PressDocument } from '../src/lib/types/document';

async function start(page: Page, doc = createDocument()) {
  await page.addInitScript(({ key, doc }) => localStorage.setItem(key, JSON.stringify(doc)), {
    key: AUTOSAVE_KEY,
    doc
  });
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Pieces', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Dismiss editor hint' }).click();
}

async function grid(page: Page) {
  await page.getByRole('button', { name: 'Grid settings', exact: true }).click();
}
async function allPieces(page: Page) {
  await page.getByRole('button', { name: 'Piece filters', exact: true }).click();
  await page.getByRole('button', { name: 'All pieces', exact: true }).click();
  await page.keyboard.press('Escape');
}
async function passActions(page: Page, name: string) {
  await page.getByRole('button', { name: `Actions for ${name}`, exact: true }).click();
}
function visibleLabel(page: Page, name: string) {
  return page.getByLabel(name, { exact: true }).filter({ visible: true });
}

async function saved(page: Page) {
  await expect(page.locator('.app-footer .local-save')).toHaveText('Saved on this device');
}
async function documentState(page: Page): Promise<PressDocument> {
  await saved(page);
  return page.evaluate((key) => JSON.parse(localStorage.getItem(key)!), AUTOSAVE_KEY);
}
async function studPoint(page: Page, x: number, y: number) {
  const box = (await page.locator('.artboard').boundingBox())!;
  const doc = await documentState(page);
  return {
    x: box.x + (x / doc.board.width) * box.width,
    y: box.y + (y / doc.board.height) * box.height
  };
}
async function clickStud(page: Page, x: number, y: number) {
  const p = await studPoint(page, x + 0.5, y + 0.5);
  await page.mouse.click(p.x, p.y);
}
async function count(page: Page, number: number) {
  await expect(page.locator('.artboard [data-uid]')).toHaveCount(number);
}
async function exportFile(
  page: Page,
  format: 'PNG' | 'SVG',
  background: boolean,
  scale = '1',
  mode: 'design' | 'print' = 'print'
) {
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  await page
    .getByRole('button', {
      name: format === 'SVG' ? 'SVG Scalable artwork' : 'PNG High-resolution print',
      exact: true
    })
    .click();
  await page.getByLabel('Export artwork mode').selectOption(mode);
  await page.getByLabel('Export resolution').selectOption(scale);
  await page.getByLabel('Include paper background').setChecked(background);
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: `Export ${format}`, exact: true }).click();
  return readFile((await (await download).path())!);
}

test('all 41 catalog pieces can be found and placed with their supplied silhouette', async ({
  page
}) => {
  const doc = createDocument();
  doc.board = { width: 48, height: 48 };
  await start(page, doc);
  await expect(page.locator('.piece-card')).toHaveCount(10);
  await allPieces(page);
  await expect(page.locator('.piece-card')).toHaveCount(41);
  let x = 1,
    y = 1,
    rowHeight = 0;
  for (const piece of pieces) {
    if (x + piece.footprint.widthStuds >= 47) {
      x = 1;
      y += rowHeight + 1;
      rowHeight = 0;
    }
    await page
      .getByRole('button', { name: `Place ${piece.name}, ${piece.designId}`, exact: true })
      .click();
    await clickStud(page, x, y);
    const placed = page.locator('.artboard [data-uid]').last().locator('path');
    await expect(placed).toHaveAttribute('d', piece.geometry.path);
    x += piece.footprint.widthStuds + 1;
    rowHeight = Math.max(rowHeight, piece.footprint.heightStuds);
  }
  await count(page, 41);
  await page.getByLabel('Search pieces').fill('27925');
  await expect(page.locator('.piece-card')).toHaveCount(1);
  await page.getByLabel('Search pieces').fill('');
});

test('placement, collision, drag, rotate, duplicate, multiselect, clipboard and history', async ({
  page
}) => {
  await start(page);
  await page.getByRole('button', { name: 'Place Tile 1×2, 3069', exact: true }).click();
  await clickStud(page, 3, 3);
  await clickStud(page, 3, 3);
  await count(page, 1);
  await expect(page.getByRole('status').filter({ hasText: 'overlaps' })).toBeVisible();
  await page.getByRole('button', { name: 'Place Tile 1×1, 3070', exact: true }).click();
  await clickStud(page, 9, 9);
  await page.keyboard.press('Escape');
  await page.locator('.artboard [data-uid]').first().locator('path').click();
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('r');
  let state = await documentState(page);
  let first = state.passes[0].pieces[0];
  expect(first).toMatchObject({ x: 4, y: 3, rotation: 90 });
  const from = await studPoint(page, 4.5, 4),
    to = await studPoint(page, 6.5, 6);
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move(to.x, to.y, { steps: 12 });
  await page.mouse.up();
  state = await documentState(page);
  expect(state.passes[0].pieces[0]).toMatchObject({ x: 6, y: 5 });
  await page.keyboard.press('Control+z');
  state = await documentState(page);
  expect(state.passes[0].pieces[0]).toMatchObject({ x: 4, y: 3 });
  await page.keyboard.press('Control+Shift+z');
  state = await documentState(page);
  expect(state.passes[0].pieces[0]).toMatchObject({ x: 6, y: 5 });
  await page.keyboard.press('Control+d');
  await count(page, 3);
  await page.keyboard.press('Escape');
  await page.locator('.artboard [data-uid]').nth(0).locator('path').click();
  await page
    .locator('.artboard [data-uid]')
    .nth(1)
    .locator('path')
    .click({ modifiers: ['Shift'] });
  await expect(page.getByText('2 pieces selected', { exact: true })).toBeVisible();
  await page.keyboard.press('Control+c');
  await page.keyboard.press('Control+v');
  await count(page, 5);
  await page.keyboard.press('Delete');
  await count(page, 3);
  await page.keyboard.press('Control+z');
  await count(page, 5);
  await page.keyboard.press('Escape');
  const boxStart = await studPoint(page, 1, 1),
    boxEnd = await studPoint(page, 13, 13);
  await page.mouse.move(boxStart.x, boxStart.y);
  await page.mouse.down();
  await page.mouse.move(boxEnd.x, boxEnd.y, { steps: 8 });
  await page.mouse.up();
  await expect(page.getByText('5 pieces selected', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Custom ink color', exact: true }).click();
  await page.getByLabel('Ink HEX color').fill('795186');
  await page.getByLabel('Ink HEX color').press('Tab');
  state = await documentState(page);
  expect(state.passes.find((p) => p.color === '#795186')?.pieces).toHaveLength(5);
});

test('physical mode, palette dragging, symmetry and pass hide/lock/recolor/reorder', async ({
  page
}) => {
  await start(page);
  await grid(page);
  await page.getByLabel('Physical print mode').check();
  await page.keyboard.press('Escape');
  await allPieces(page);
  await expect(
    page.getByRole('button', { name: 'Place Tile 4×4×⅔ with Rounded Corners, 68869', exact: true })
  ).toBeDisabled();
  const boardBox = (await page.locator('.artboard').boundingBox())!;
  await page
    .getByRole('button', { name: 'Place Tile 1×1, 3070', exact: true })
    .dragTo(page.locator('.artboard'), {
      targetPosition: { x: (boardBox.width * 5.5) / 16, y: (boardBox.height * 5.5) / 16 }
    });
  await count(page, 1);
  await grid(page);
  await page.getByLabel('Symmetry', { exact: true }).selectOption('both');
  await page.keyboard.press('Escape');
  await page
    .getByRole('button', { name: 'Place Tile Round 1×1 Quarter, 25269', exact: true })
    .click();
  await clickStud(page, 2, 2);
  await count(page, 5);
  let state = await documentState(page);
  expect(state.passes[0].pieces.filter((p) => p.mirrorX || p.mirrorY)).toHaveLength(3);
  await page.keyboard.press('Escape');
  await page.getByRole('tab', { name: 'Layers', exact: true }).click();
  await page.getByRole('button', { name: 'Hide 01 · Vermilion', exact: true }).click();
  await count(page, 0);
  await page.getByRole('button', { name: 'Show 01 · Vermilion', exact: true }).click();
  await count(page, 5);
  await passActions(page, '01 · Vermilion');
  await page.getByRole('button', { name: 'Lock 01 · Vermilion', exact: true }).click();
  await page.locator('.artboard [data-uid]').first().locator('path').click();
  await expect(page.getByText('0 selected', { exact: false }).last()).toBeVisible();
  await passActions(page, '01 · Vermilion');
  await page.getByRole('button', { name: 'Unlock 01 · Vermilion', exact: true }).click();
  await page.getByRole('button', { name: 'Expand 01 · Vermilion', exact: true }).click();
  await visibleLabel(page, 'Rename ink pass 1').fill('First impression');
  await visibleLabel(page, 'Rename ink pass 1').press('Tab');
  await passActions(page, 'First impression');
  await visibleLabel(page, 'Recolor First impression').fill('#315eae');
  await visibleLabel(page, 'Recolor First impression').press('Tab');
  await page.getByRole('button', { name: 'Raise First impression', exact: true }).click();
  state = await documentState(page);
  expect(state.passes[1]).toMatchObject({
    name: 'First impression',
    color: '#315eae',
    locked: false
  });
});

test('printing is deterministic, registration applies to passes, and presets/reseed undo correctly', async ({
  page
}) => {
  const doc = createDocument(true);
  await start(page, doc);
  await page.getByRole('button', { name: 'Print preview', exact: true }).click();
  await page.getByText('Advanced print settings', { exact: true }).click();
  await expect(page.getByLabel('Print preset')).toHaveValue('Normal');
  const visible = doc.passes.flatMap((p) => p.pieces).length;
  await expect(page.locator('.artboard [data-uid] filter')).toHaveCount(visible);
  const original = await page.locator('.artboard [data-uid]').first().getAttribute('transform');
  await page.getByRole('slider', { name: 'Registration error', exact: true }).press('End');
  const transforms = await page
    .locator('.artboard [data-pass]')
    .evaluateAll((nodes) => nodes.map((n) => n.getAttribute('transform')));
  expect(new Set(transforms).size).toBe(3);
  await page.getByRole('button', { name: 'Reseed print', exact: true }).click();
  expect(await page.locator('.artboard [data-uid]').first().getAttribute('transform')).not.toBe(
    original
  );
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  expect(await page.locator('.artboard [data-uid]').first().getAttribute('transform')).toBe(
    original
  );
  await page.getByLabel('Print preset').selectOption('Dry Ink');
  const state = await documentState(page);
  expect(state.printSettings.inkAmount).toBe(0.46);
  await page.screenshot({ path: test.info().outputPath('print-preview.png') });
});

test('projects save, load and recover complete document data; vector and 4× transparent PNG export', async ({
  page
}) => {
  page.on('dialog', (dialog) => dialog.accept());
  await start(page);
  await page.getByRole('button', { name: 'Place Tile 1×1, 3070', exact: true }).click();
  await clickStud(page, 3, 3);
  await clickStud(page, 4, 3);
  const original = await documentState(page);
  const projectDownload = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  const project = await readFile((await (await projectDownload).path())!, 'utf8');
  expect(JSON.parse(project)).toEqual(original);
  await page.getByRole('button', { name: 'New', exact: true }).click();
  await count(page, 0);
  await page.getByLabel('Open project file').setInputFiles({
    name: 'saved.legopress.json',
    mimeType: 'application/json',
    buffer: Buffer.from(project)
  });
  await count(page, 2);
  expect(await documentState(page)).toEqual(original);
  // Disable the fixture init script for recovery by loading in a fresh tab sharing storage.
  const recovered = await page.context().newPage();
  await recovered.goto('/');
  await count(recovered, 2);
  await recovered.close();
  const svg = (await exportFile(page, 'SVG', false, '1', 'design')).toString('utf8');
  expect(svg.match(/<path data-piece-id=/g)).toHaveLength(2);
  expect(svg).not.toContain('filter=');
  expect(svg).not.toContain('stroke-dasharray');
  expect(svg).not.toContain('export-grid');
  const png = await exportFile(page, 'PNG', false, '4');
  expect(png.readUInt32BE(16)).toBe(4096);
  expect(png.readUInt32BE(20)).toBe(4096);
  const cornerAlpha = await page.evaluate(async (base64) => {
    const image = new Image();
    image.src = 'data:image/png;base64,' + base64;
    await image.decode();
    const canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext('2d')!;
    context.drawImage(image, 0, 0);
    return context.getImageData(0, 0, 1, 1).data[3];
  }, png.toString('base64'));
  expect(cornerAlpha).toBe(0);
  const first = await exportFile(page, 'PNG', true, '1');
  const second = await exportFile(page, 'PNG', true, '1');
  expect(first.equals(second)).toBe(true);
});

test('2,000 individual pieces remain editable, and a group move makes one history transaction', async ({
  page
}) => {
  const doc = createDocument();
  doc.board = { width: 48, height: 48 };
  doc.passes = [makePass('#252a26')];
  doc.passes[0].pieces = Array.from({ length: 2000 }, (_, i) => ({
    uid: `perf-${i}`,
    pieceId: '3070',
    x: i % 48,
    y: Math.floor(i / 48),
    rotation: 0,
    seed: i + 1
  }));
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await start(page, doc);
  await count(page, 2000);
  await page.locator('.canvas-workspace').focus();
  await page.keyboard.press('Control+a');
  const started = Date.now();
  await page.keyboard.press('ArrowDown');
  let state = await documentState(page);
  expect(state.passes[0].pieces[0].y).toBe(1);
  expect(state.passes[0].pieces).toHaveLength(2000);
  test.info().annotations.push({
    type: 'performance',
    description: `2,000-piece selection and committed group move: ${Date.now() - started} ms, including autosave.`
  });
  await page.keyboard.press('Control+z');
  state = await documentState(page);
  expect(state.passes[0].pieces[0].y).toBe(0);
  expect(errors).toEqual([]);
});

test('grid settings and responsive drawers stay usable at narrow widths', async ({ page }) => {
  await start(page);
  await grid(page);
  await expect(page.getByLabel('Major grid interval')).toHaveValue('4');
  await page.getByLabel('Major grid interval').selectOption('8');
  await expect(page.getByLabel('Major grid interval')).toHaveValue('8');
  await page.keyboard.press('Escape');
  for (const width of [1200, 1000, 900, 390, 320]) {
    await page.setViewportSize({ width, height: 844 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(width);
    await expect(page.locator('.canvas-workspace')).toBeVisible();
    if (width < 1000) {
      const trigger = page.getByRole('button', { name: 'Open Pieces', exact: true });
      await trigger.click();
      const drawer = page.getByRole('dialog', { name: 'Pieces', exact: true });
      await expect(drawer).toBeVisible();
      await page.keyboard.press('Escape');
      await expect(drawer).not.toBeVisible();
      await expect(trigger).toBeFocused();
      await page.getByRole('button', { name: 'Open Properties', exact: true }).click();
      await expect(page.getByRole('tab', { name: 'Properties', exact: true })).toBeVisible();
      await page.keyboard.press('Escape');
    }
  }
  await page.getByRole('button', { name: 'Open Pieces', exact: true }).click();
  await page.getByRole('button', { name: 'Place Tile 1×1, 3070', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('dialog', { name: 'Pieces', exact: true })).not.toBeVisible();
  await expect(page.locator('.canvas-workspace')).toBeFocused();
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  const mobileDocument = await documentState(page);
  expect(mobileDocument.passes[0].pieces[0]).toMatchObject({ x: 1, y: 1 });
  await page.screenshot({ path: 'artifacts/reference-mobile.png' });
});

test('first launch hydrates stable starter pieces and can select without autosave recovery', async ({
  page
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'warning' && message.text().includes('hydration'))
      errors.push(message.text());
  });
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Pieces', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Dismiss editor hint' }).click();
  await page.locator('.artboard [data-uid]').first().locator('path').click();
  await expect(page.getByText('Selected piece', { exact: true })).toBeVisible();
  await expect(page.getByRole('toolbar', { name: 'Selection actions' })).toBeVisible();
  expect(errors).toEqual([]);
  await page.screenshot({ path: test.info().outputPath('design-studio.png') });
});

test('rejected physical/overlap/resize changes leave controls accurate, and physical mode guards the clipboard', async ({
  page
}) => {
  page.on('dialog', (dialog) => dialog.accept());
  const doc = createDocument();
  doc.passes[0].pieces = [{ uid: 'special', pieceId: '68869', x: 0, y: 0, rotation: 0, seed: 1 }];
  await start(page, doc);
  await grid(page);
  await page.getByLabel('Physical print mode').click();
  await expect(page.getByLabel('Physical print mode')).not.toBeChecked();
  await page.keyboard.press('Escape');
  await page.locator('.artboard [data-uid]').first().locator('path').click();
  await page.keyboard.press('Control+c');
  await page.keyboard.press('Delete');
  await grid(page);
  await page.getByLabel('Physical print mode').check();
  await page.keyboard.press('Escape');
  await page.locator('.canvas-workspace').focus();
  await page.keyboard.press('Control+v');
  await count(page, 0);
  await grid(page);
  await page.getByLabel('Allow overlap').check();
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Place Tile 1×1, 3070', exact: true }).click();
  await clickStud(page, 14, 14);
  await clickStud(page, 14, 14);
  await count(page, 2);
  await page.keyboard.press('Escape');
  await grid(page);
  await page.getByLabel('Allow overlap').click();
  await expect(page.getByLabel('Allow overlap')).toBeChecked();
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Artboard size', exact: true }).click();
  await visibleLabel(page, 'Artboard preset').selectOption('8');
  await expect(visibleLabel(page, 'Artboard preset')).toHaveValue('16');
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Main menu', exact: true }).click();
  await page.getByRole('button', { name: 'Clear canvas', exact: true }).click();
  await count(page, 0);
  await grid(page);
  await page.getByLabel('Allow overlap').uncheck();
  await expect(page.getByLabel('Allow overlap')).not.toBeChecked();
});

test('keyboard placement, active ink thumbnails, tabs and synchronized zoom', async ({ page }) => {
  await start(page);
  const tile = page.getByRole('button', { name: 'Place Tile 1×1, 3070', exact: true });
  await expect(tile.locator('path')).toHaveAttribute('fill', '#ce4936');
  await tile.focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('.canvas-workspace')).toBeFocused();
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  let state = await documentState(page);
  expect(state.passes[0].pieces[0]).toMatchObject({ x: 1, y: 1 });
  await page.keyboard.press('Escape');
  await clickStud(page, 1, 1);
  const selected = await documentState(page);
  await page.getByRole('tab', { name: 'Properties', exact: true }).focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('tab', { name: 'Layers', exact: true })).toBeFocused();
  await expect(page.getByRole('tab', { name: 'Layers', exact: true })).toHaveAttribute(
    'aria-selected',
    'true'
  );
  expect((await documentState(page)).passes).toEqual(selected.passes);
  await page.keyboard.press('End');
  await expect(page.getByRole('tab', { name: 'Export', exact: true })).toBeFocused();
  await page.getByLabel('Export resolution').selectOption('4');
  await page.getByRole('tab', { name: 'Properties', exact: true }).click();
  await page.getByRole('tab', { name: 'Export', exact: true }).click();
  await expect(page.getByLabel('Export resolution')).toHaveValue('4');
  await page.getByRole('button', { name: 'Toolbar zoom in', exact: true }).click();
  const percentages = await page.locator('.zoom-value').allTextContents();
  expect(percentages).toHaveLength(2);
  expect(percentages[0]).toEqual(percentages[1]);
});

test('embossed guide persists independently and never enters the document or exports', async ({
  page
}) => {
  await start(page);
  const before = await documentState(page);
  await grid(page);
  await page.getByLabel('Grid appearance').selectOption('embossed');
  await page.keyboard.press('Escape');
  await expect(page.locator('[data-editor-guide="embossed"]')).toBeVisible();
  expect(await documentState(page)).toEqual(before);
  const flatSvg = (await exportFile(page, 'SVG', true, '1', 'design')).toString('utf8');
  expect(flatSvg).not.toContain('embossed');
  await page.reload();
  await expect(page.locator('[data-editor-guide="embossed"]')).toBeVisible();
  await page.getByRole('button', { name: 'Print preview', exact: true }).click();
  await expect(page.locator('[data-editor-guide="embossed"]')).toHaveCount(0);
});

test('popovers dismiss and restore focus; keyboard hand panning changes only the view', async ({
  page
}) => {
  await start(page);
  const trigger = page.getByRole('button', { name: 'Grid settings', exact: true });
  await trigger.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('dialog', { name: 'Grid settings', exact: true })).toBeVisible();
  await page.getByLabel('Grid style').focus();
  await page.keyboard.press('Escape');
  await expect(trigger).toBeFocused();
  const before = await documentState(page);
  const art = page.locator('.artboard-transform');
  const transform = await art.getAttribute('style');
  await page.locator('.canvas-workspace').focus();
  await page.keyboard.press('h');
  await page.keyboard.press('ArrowRight');
  expect(await art.getAttribute('style')).not.toBe(transform);
  expect(await documentState(page)).toEqual(before);
});

test('WCAG checks pass for both modes, inspector tabs, menus and narrow drawers', async ({
  page
}) => {
  await start(page, createDocument(true));
  async function audit() {
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'])
      .analyze();
    expect(
      results.violations.map((v) => ({ id: v.id, nodes: v.nodes.map((n) => n.target) }))
    ).toEqual([]);
  }
  for (const mode of ['Design', 'Print preview']) {
    await page.getByRole('button', { name: mode, exact: true }).click();
    for (const tab of ['Properties', 'Layers', 'Export']) {
      await page.getByRole('tab', { name: tab, exact: true }).click();
      await audit();
    }
  }
  await grid(page);
  await audit();
  await page.keyboard.press('Escape');
  await page.getByRole('tab', { name: 'Properties', exact: true }).click();
  await page.getByRole('button', { name: 'Custom ink color', exact: true }).click();
  await audit();
  await page.keyboard.press('Escape');
  await page.setViewportSize({ width: 320, height: 844 });
  await audit();
  for (const control of ['Open Pieces', 'Open Properties']) {
    await page.getByRole('button', { name: control, exact: true }).click();
    await audit();
    await page.keyboard.press('Escape');
  }
});

test('reference screenshots and enlarged text preserve accessible controls', async ({ page }) => {
  await page.setViewportSize({ width: 1448, height: 1086 });
  await start(page, createDocument(true));
  await expect(page.locator('.app-header')).toHaveCSS('height', '64px');
  // Paneforge rounds its percentage layout to one decimal place.
  expect(Math.abs((await page.locator('.palette-shell').boundingBox())!.width - 306)).toBeLessThan(
    1
  );
  expect(
    Math.abs((await page.locator('.inspector-shell').boundingBox())!.width - 340)
  ).toBeLessThan(1);
  await page.mouse.move(700, 80);
  await expect(page.locator('.toast')).toHaveCount(0);
  await page.screenshot({ path: 'artifacts/reference-design.png' });
  await page.getByRole('button', { name: 'Print preview', exact: true }).click();
  await page.screenshot({ path: 'artifacts/reference-print.png' });
  await page.addStyleTag({
    content:
      'button,input,select,summary,h2,h3,h4,p,span,label {line-height:1.5 !important;letter-spacing:.12em !important;word-spacing:.16em !important}'
  });
  await page.evaluate(() => {
    const sizes = Array.from(
      document.querySelectorAll<HTMLElement>('button,input,select,summary,h2,h3,h4,p,span,label')
    ).map((element) => ({ element, size: parseFloat(getComputedStyle(element).fontSize) * 2 }));
    for (const { element, size } of sizes) element.style.fontSize = `${size}px`;
  });
  await page.getByRole('button', { name: 'Grid settings', exact: true }).click();
  await expect(page.getByLabel('Physical print mode')).toBeVisible();
  await page.keyboard.press('Escape');
  for (const tab of await page.getByRole('tab').all()) {
    expect(await tab.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(true);
  }
  await page.screenshot({ path: 'artifacts/reference-enlarged-text.png' });
  await page.setViewportSize({ width: 640, height: 540 });
  await page.getByRole('button', { name: 'Open Properties', exact: true }).click();
  await expect(page.getByLabel('Paper stock')).toBeVisible();
});

test('resizable panels and drawer popovers preserve focus, choices and document data', async ({
  page
}) => {
  await page.setViewportSize({ width: 1448, height: 1086 });
  await start(page);
  const before = await documentState(page);
  const handle = page.getByRole('separator', { name: 'Resize Pieces panel', exact: true });
  const initialWidth = (await page.locator('.palette-shell').boundingBox())!.width;
  const box = (await handle.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + 55, box.y + box.height / 2, { steps: 6 });
  await page.mouse.up();
  const draggedWidth = (await page.locator('.palette-shell').boundingBox())!.width;
  expect(draggedWidth).toBeGreaterThan(initialWidth + 45);
  await handle.focus();
  await page.keyboard.press('ArrowLeft');
  expect((await page.locator('.palette-shell').boundingBox())!.width).toBeLessThan(
    draggedWidth - 20
  );
  await page.getByRole('button', { name: 'Main menu', exact: true }).click();
  await page.getByRole('button', { name: 'Reset panel widths', exact: true }).click();
  await expect
    .poll(async () =>
      Math.abs((await page.locator('.palette-shell').boundingBox())!.width - initialWidth)
    )
    .toBeLessThan(1);
  await page.getByRole('button', { name: 'Export', exact: true }).click();
  await page.getByLabel('Export resolution').selectOption('4');
  await page.setViewportSize({ width: 320, height: 844 });
  const properties = page.getByRole('button', { name: 'Open Properties', exact: true });
  await properties.click();
  await expect(page.getByLabel('Export resolution')).toHaveValue('4');
  await page.getByRole('tab', { name: 'Properties', exact: true }).click();
  const ink = page.getByRole('button', { name: 'Custom ink color', exact: true });
  await ink.click();
  await expect(page.getByRole('dialog', { name: 'Custom ink color', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(ink).toBeFocused();
  await expect(page.getByRole('dialog', { name: 'Inspector', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(properties).toBeFocused();
  await page.getByRole('button', { name: 'Open Pieces', exact: true }).click();
  await page.getByRole('button', { name: 'Place Tile 1×1, 3070', exact: true }).click();
  await expect(page.locator('.canvas-workspace')).toBeFocused();
  await page.keyboard.press('Enter');
  await count(page, 1);
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Open Properties', exact: true }).click();
  await page.setViewportSize({ width: 1448, height: 1086 });
  await expect(page.locator('.inspector-shell')).toBeVisible();
  await page.getByRole('tab', { name: 'Export', exact: true }).click();
  await expect(page.getByLabel('Export resolution')).toHaveValue('4');
  const after = await documentState(page);
  expect(after.board).toEqual(before.board);
  expect(after.passes.map((p) => ({ ...p, pieces: [] }))).toEqual(before.passes);
});

test('piece tooltips expose catalog metadata on keyboard focus and dismiss with Escape', async ({
  page
}) => {
  await start(page);
  const tile = page.getByRole('button', { name: 'Place Tile 1×1, 3070', exact: true });
  await tile.focus();
  const tooltip = page.getByRole('tooltip');
  await expect(tooltip).toBeVisible();
  await expect(tooltip).toContainText('3070 · 1 × 1 studs');
  await expect(tooltip).toContainText('Catalog silhouette · 1 plate height');
  await expect(tile).toHaveAttribute('aria-describedby', (await tooltip.getAttribute('id'))!);
  const audit = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'])
    .analyze();
  expect(audit.violations.map((violation) => violation.id)).toEqual([]);
  await page.keyboard.press('Escape');
  await expect(tooltip).toHaveCount(0);
  await expect(tile).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.locator('.canvas-workspace')).toBeFocused();
  await grid(page);
  await page.getByLabel('Physical print mode').check();
  await page.keyboard.press('Escape');
  await allPieces(page);
  const incompatible = page.getByRole('button', {
    name: 'Place Tile 4×4×⅔ with Rounded Corners, 68869',
    exact: true
  });
  await incompatible.focus();
  await expect(page.getByRole('tooltip')).toContainText('Non-standard printing height');
  await page.keyboard.press('Enter');
  await expect(incompatible).toBeFocused();
  await count(page, 0);
});

test('a quarter tile fits a quarter-ring cutout while real overlaps remain blocked', async ({
  page
}) => {
  const doc = createDocument();
  doc.name = 'Quarter-ring fit';
  doc.board = { width: 8, height: 8 };
  doc.options.grid = 'squares';
  doc.passes[0].pieces = [{ uid: 'ring', pieceId: '27925', x: 3, y: 3, rotation: 0, seed: 171 }];
  const inset = makePass('#005ac5', 'Quarter inset');
  doc.passes.push(inset);
  await start(page, doc);
  await page.getByRole('button', { name: 'Use Quarter inset', exact: true }).click();
  await page
    .getByRole('button', { name: 'Place Tile Round 1×1 Quarter, 25269', exact: true })
    .click();
  await clickStud(page, 3, 3);
  await count(page, 2);
  const fitted = await documentState(page);
  expect(fitted.options.allowOverlap).toBe(false);
  expect(fitted.passes.find((pass) => pass.id === inset.id)!.pieces[0]).toMatchObject({
    pieceId: '25269',
    x: 3,
    y: 3,
    rotation: 0
  });
  // Check the actual browser SVG fills, independently of the collision engine.
  const sharedInterior = await page.evaluate(() => {
    const svg = document.querySelector<SVGSVGElement>('.artboard')!;
    const paths = Array.from(svg.querySelectorAll<SVGPathElement>('[data-uid] path'));
    const world = svg.getScreenCTM()!;
    const inverse = paths.map((path) => path.getScreenCTM()!.inverse());
    let shared = 0;
    for (let y = 0; y < 100; y++)
      for (let x = 0; x < 100; x++) {
        const point = new DOMPoint(3 + (x + 0.5) / 50, 3 + (y + 0.5) / 50).matrixTransform(world);
        if (paths.every((path, i) => path.isPointInFill(point.matrixTransform(inverse[i]))))
          shared++;
      }
    return shared;
  });
  expect(sharedInterior).toBe(0);
  await page.keyboard.press('Escape');
  await page.mouse.move(700, 80);
  await page.screenshot({ path: 'artifacts/quarter-ring-fit.png' });
  await page.getByRole('button', { name: 'Place Tile 1×1, 3070', exact: true }).click();
  await clickStud(page, 4, 3);
  await expect(page.locator('.toast')).toContainText('overlaps');
  await count(page, 2);
  await page.keyboard.press('Control+z');
  await count(page, 1);
  await page.keyboard.press('Control+Shift+z');
  await count(page, 2);
  await grid(page);
  await page.getByLabel('Allow overlap').check();
  await page.getByLabel('Allow overlap').uncheck();
  await expect(page.getByLabel('Allow overlap')).not.toBeChecked();
  await page.keyboard.press('Escape');
  await saved(page);
  // A fresh tab recovers storage without re-running this page's fixture init script.
  const recovered = await page.context().newPage();
  await recovered.goto('/');
  await count(recovered, 2);
  expect((await documentState(recovered)).options.allowOverlap).toBe(false);
  await recovered.close();
});

test('design seams have a constant width across sizes, curves and diagonals', async ({ page }) => {
  const doc = createDocument();
  doc.name = 'Consistent design spacing';
  doc.board = { width: 10, height: 8 };
  doc.options.grid = 'off';
  const poses = [
    ['3070', 1, 1, 0],
    ['2431', 2, 1, 0],
    ['3070', 6, 1, 0],
    ['3068', 2, 2, 0],
    ['3068', 4, 2, 0],
    ['2431', 6, 2, 90],
    ['27925', 2, 4, 0],
    ['25269', 2, 4, 0],
    ['35787', 8, 4, 0],
    ['35787', 8, 4, 180]
  ] as const;
  doc.passes[0].pieces = poses.map(([pieceId, x, y, rotation], i) => ({
    uid: `seam-${i}`,
    pieceId,
    x,
    y,
    rotation,
    seed: 171 + i
  }));
  await start(page, doc);
  await count(page, poses.length);
  await page.screenshot({ path: 'artifacts/design-spacing.png' });
  // Rasterize only the live artwork and its definitions, with transparent seams.
  const live = await page.locator('.artboard').evaluate((svg) => {
    const clone = svg.cloneNode(true) as SVGSVGElement;
    for (const child of Array.from(clone.children))
      if (child.tagName !== 'defs' && !child.hasAttribute('data-pass')) child.remove();
    clone.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
    return clone.outerHTML;
  });
  const vector = (await exportFile(page, 'SVG', false, '4', 'design')).toString('utf8');
  const png = (await exportFile(page, 'PNG', false, '4', 'design')).toString('base64');
  const measurements = await page.evaluate(
    async ({ live, vector, png }) => {
      const samples = [
        { x: 2, y: 1.5, dx: 1, dy: 0 },
        { x: 6, y: 1.5, dx: 1, dy: 0 },
        { x: 2.7, y: 2, dx: 0, dy: 1 },
        { x: 4, y: 3, dx: 1, dy: 0 },
        { x: 6.5, y: 2, dx: 0, dy: 1 },
        { x: 6, y: 3, dx: 1, dy: 0 },
        ...[20, 45, 70].map((degrees) => {
          const angle = (degrees * Math.PI) / 180;
          return {
            x: 2 + Math.cos(angle),
            y: 4 + Math.sin(angle),
            dx: Math.cos(angle),
            dy: Math.sin(angle)
          };
        }),
        { x: 9, y: 5, dx: Math.SQRT1_2, dy: -Math.SQRT1_2 }
      ];
      const results: number[][] = [];
      for (const [index, markup] of [live, vector, png].entries()) {
        const image = new Image();
        const url =
          index === 2
            ? 'data:image/png;base64,' + markup
            : URL.createObjectURL(new Blob([markup], { type: 'image/svg+xml' }));
        image.src = url;
        await image.decode();
        const canvas = document.createElement('canvas');
        canvas.width = 2560;
        canvas.height = 2048;
        const context = canvas.getContext('2d')!;
        context.drawImage(image, 0, 0, canvas.width, canvas.height);
        const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
        results.push(
          samples.map(({ x, y, dx, dy }) => {
            const transparent: number[] = [];
            for (let step = -160; step <= 160; step++) {
              const distance = step / 2048;
              const px = Math.floor((x + dx * distance) * 256);
              const py = Math.floor((y + dy * distance) * 256);
              if (pixels[(py * canvas.width + px) * 4 + 3] < 128) transparent.push(distance);
            }
            return transparent.length ? transparent.at(-1)! - transparent[0] + 1 / 2048 : 0;
          })
        );
        if (index !== 2) URL.revokeObjectURL(url);
      }
      return results;
    },
    { live, vector, png }
  );
  for (const gaps of measurements)
    for (const gap of gaps) expect(Math.abs(gap - 0.025)).toBeLessThan(0.006);
  expect(await documentState(page)).toEqual(doc);
});
