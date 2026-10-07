import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { createDocument, AUTOSAVE_KEY } from '../src/lib/persistence/document';
import { pieces } from '../src/lib/catalog/catalog';

const preferencesKey = 'form-impression-grid-appearance';
async function start(page: Page) {
  await page.addInitScript(({ key, doc }) => localStorage.setItem(key, JSON.stringify(doc)), {
    key: AUTOSAVE_KEY,
    doc: createDocument(true)
  });
  await page.goto('/');
  await expect(page.locator('.editor-app')).toHaveAttribute('data-ready', 'true');
  await page.getByRole('button', { name: 'Dismiss editor hint' }).click();
}
async function preference(page: Page) {
  return page.evaluate((key) => JSON.parse(localStorage.getItem(key)!), preferencesKey);
}

test('export preview follows the exact SVG settings and choices survive tabs', async ({ page }) => {
  await start(page);
  await page.getByRole('tab', { name: 'Export', exact: true }).click();
  const preview = page.locator('.export-preview img');
  await expect(preview).toBeVisible();
  await expect
    .poll(() =>
      preview.evaluate((image: HTMLImageElement) => image.complete && image.naturalWidth > 0)
    )
    .toBe(true);
  await page.getByLabel('Export artwork mode', { exact: true }).selectOption('print');
  await page.getByLabel('Include paper background', { exact: true }).uncheck();
  await page.getByLabel('Include grid in export', { exact: true }).check();
  await expect(page.locator('.export-preview figcaption')).toContainText(
    'Print impression · Transparent'
  );
  await expect
    .poll(async () => {
      try {
        return await preview.evaluate(async (image: HTMLImageElement) =>
          (await (await fetch(image.src)).text()).includes('export-grid')
        );
      } catch {
        return false;
      }
    })
    .toBe(true);
  const svg = await preview.evaluate(async (image: HTMLImageElement) =>
    (await fetch(image.src)).text()
  );
  expect(svg).toContain('Seeded procedural print filters.');
  expect(svg).not.toContain('paper-grain');
  expect(svg).not.toContain('data-tracing-image');
  await page.getByRole('tab', { name: 'Layers', exact: true }).click();
  await page.getByRole('tab', { name: 'Export', exact: true }).click();
  await expect(page.getByLabel('Export artwork mode', { exact: true })).toHaveValue('print');
  await expect(page.getByLabel('Include paper background', { exact: true })).not.toBeChecked();
  await expect(page.getByLabel('Include grid in export', { exact: true })).toBeChecked();
  await page.getByLabel('Export resolution', { exact: true }).selectOption('custom');
  await page.getByLabel('Export pixel width', { exact: true }).fill('');
  await expect(page.getByRole('button', { name: 'Export PNG', exact: true })).toBeDisabled();
  await page.getByLabel('Export pixel width', { exact: true }).fill('13000');
  await expect(page.getByRole('button', { name: 'Export PNG', exact: true })).toBeDisabled();
  await expect(page.locator('.export-preview')).toContainText('12,000');
});

test('print settings belong to preview and custom presets recover all controls', async ({
  page
}) => {
  await start(page);
  await expect(page.locator('.piece-card')).toHaveCount(pieces.length);
  await expect(page.getByRole('button', { name: 'Essentials', exact: true })).toHaveCount(0);
  await expect(page.getByRole('slider', { name: 'Pressure', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Print preview', exact: true }).click();
  await page.getByRole('slider', { name: 'Pressure', exact: true }).press('Home');
  await page.getByRole('slider', { name: 'Pressure', exact: true }).press('ArrowRight');
  await page.getByText('Advanced print settings', { exact: true }).click();
  await page.getByRole('slider', { name: 'Piece wear', exact: true }).press('End');
  await page.getByLabel('Print preset name', { exact: true }).fill('My textured press');
  await page.locator('.preset-save-row button').click();
  await expect.poll(async () => (await preference(page)).printPresets.length).toBe(1);
  const saved = (await preference(page)).printPresets[0];
  expect(saved.settings.pressure).toBe(0.01);
  expect(saved.settings.pieceWear).toBe(1);
  await page.reload();
  await page.getByRole('button', { name: 'Print preview', exact: true }).click();
  await page.getByLabel('Print preset', { exact: true }).selectOption(saved.id);
  await expect(page.getByRole('slider', { name: 'Pressure', exact: true })).toHaveValue('0.01');
  await page.getByText('Advanced print settings', { exact: true }).click();
  await expect(page.getByRole('slider', { name: 'Piece wear', exact: true })).toHaveValue('1');
  await page.getByRole('button', { name: 'Design', exact: true }).click();
  await expect(page.getByRole('slider', { name: 'Pressure', exact: true })).toHaveCount(0);
});

test('ink passes drag by insertion position, retain active ink and undo; keyboard reorder works', async ({
  page
}) => {
  await start(page);
  await page.getByRole('tab', { name: 'Layers', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Add ink pass', exact: true })).toHaveCount(1);
  const rows = page.locator('.pass-block');
  const first = page.getByRole('button', { name: 'Reorder 01 · Vermilion', exact: true });
  const last = rows.last();
  const from = (await first.boundingBox())!,
    target = (await last.boundingBox())!;
  await page.mouse.move(from.x + 12, from.y + 12);
  await page.mouse.down();
  await page.mouse.move(from.x + 20, from.y + 18);
  await page.mouse.move(target.x + 100, target.y + target.height - 2, { steps: 10 });
  await page.mouse.up();
  await expect(rows.last().locator('.pass-name')).toHaveValue('01 · Vermilion');
  await expect(
    page.getByRole('button', { name: 'Use 01 · Vermilion', exact: true })
  ).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(rows.first().locator('.pass-name')).toHaveValue('01 · Vermilion');
  await first.focus();
  await page.keyboard.press('ArrowDown');
  await expect(rows.nth(1).locator('.pass-name')).toHaveValue('01 · Vermilion');
  await expect(first).toBeFocused();
});

test('fixed editor panels and export preview have no serious accessibility issues', async ({
  page
}) => {
  await start(page);
  await expect(page.locator('.palette-shell')).toBeVisible();
  await expect(page.locator('.inspector-shell')).toBeVisible();
  await page.getByRole('tab', { name: 'Export', exact: true }).click();
  await expect(page.locator('.export-preview img')).toBeVisible();
  expect(
    (await new AxeBuilder({ page }).analyze()).violations.filter((v) =>
      ['serious', 'critical'].includes(v.impact ?? '')
    )
  ).toEqual([]);
});
