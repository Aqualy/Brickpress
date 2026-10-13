import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { mkdir } from 'node:fs/promises';
import { createDocument, AUTOSAVE_KEY } from '../src/lib/persistence/document';

async function start(page: Page) {
  const doc = createDocument(true);
  await page.addInitScript(({ key, doc }) => localStorage.setItem(key, JSON.stringify(doc)), {
    key: AUTOSAVE_KEY,
    doc
  });
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Select tool', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Dismiss editor hint' }).click();
  const dismiss = page.getByRole('button', { name: 'Dismiss notification', exact: true });
  if (await dismiss.isVisible()) await dismiss.click();
  await expect(page.locator('.local-save')).toHaveText('Saved on this device');
}
async function accessible(page: Page) {
  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'])
    .analyze();
  expect(
    results.violations.map((v) => ({
      id: v.id,
      nodes: v.nodes.map((n) => ({ target: n.target, summary: n.failureSummary }))
    }))
  ).toEqual([]);
}

for (const colorScheme of ['light', 'dark'] as const) {
  test(`${colorScheme} appearance: all editor views and inspector tabs remain accessible`, async ({
    page
  }) => {
    test.setTimeout(150_000);
    await page.emulateMedia({ colorScheme });
    await page.setViewportSize({ width: 1448, height: 1086 });
    await start(page);
    const before = await page.evaluate((key) => localStorage.getItem(key), AUTOSAVE_KEY);
    await mkdir('artifacts/appearance', { recursive: true });
    await page.screenshot({ path: `artifacts/appearance/${colorScheme}-design.png` });
    for (const mode of ['Design', 'Print preview', '3D view']) {
      await page.getByRole('button', { name: mode, exact: true }).click();
      if (mode === '3D view')
        await expect(page.locator('.three-canvas')).toHaveAttribute('data-ready', 'true', {
          timeout: 30000
        });
      for (const tab of ['Properties', 'Layers', 'Export']) {
        await page.getByRole('tab', { name: tab, exact: true }).click();
        await accessible(page);
      }
    }
    await page.emulateMedia({ colorScheme: colorScheme === 'light' ? 'dark' : 'light' });
    expect(await page.evaluate((key) => localStorage.getItem(key), AUTOSAVE_KEY)).toBe(before);
    await page.getByRole('tab', { name: 'Properties', exact: true }).click();
    await page.screenshot({ path: `artifacts/appearance/${colorScheme}-three-opposite-theme.png` });
  });

  test(`${colorScheme} menus, focus restoration and 320px drawers`, async ({ page }) => {
    await page.emulateMedia({ colorScheme, reducedMotion: 'reduce', contrast: 'more' });
    await start(page);
    await page.getByRole('button', { name: 'Main menu', exact: true }).click();
    await accessible(page);
    await page.getByRole('button', { name: 'Switch to Place', exact: true }).click();
    await expect(page.locator('.canvas-workspace')).toBeFocused();
    await expect(page.getByRole('button', { name: 'Place tool', exact: true })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await page.getByRole('button', { name: 'Grid settings', exact: true }).click();
    await accessible(page);
    await page.keyboard.press('Escape');
    await expect(page.getByRole('button', { name: 'Grid settings', exact: true })).toBeFocused();
    await page.setViewportSize({ width: 320, height: 640 });
    await page.getByRole('button', { name: 'Open Pieces', exact: true }).click();
    await accessible(page);
    await page.keyboard.press('Escape');
    await expect(page.getByRole('button', { name: 'Open Pieces', exact: true })).toBeFocused();
    await page.getByRole('button', { name: 'Open Inspector', exact: true }).click();
    for (const tab of ['Properties', 'Layers', 'Export']) {
      await page.getByRole('tab', { name: tab, exact: true }).click();
      await accessible(page);
    }
    await page.keyboard.press('Escape');
    await expect(page.getByRole('button', { name: 'Open Inspector', exact: true })).toBeFocused();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
      320
    );
    await page.screenshot({ path: `artifacts/appearance/${colorScheme}-narrow.png` });
  });
}

test('notification stays readable while hovered or focused and dismisses with focus restored', async ({
  page
}) => {
  await start(page);
  await page.locator('.canvas-workspace').focus();
  await page.keyboard.press('Control+a');
  await page.keyboard.press('Control+c');
  // Copying reports completion without changing artwork.
  const notification = page.getByRole('group', { name: 'Notification', exact: true });
  await expect(notification).toBeVisible();
  await notification.hover();
  await page.waitForTimeout(3500);
  await expect(notification).toBeVisible();
  await page.locator('.canvas-workspace').focus();
  await page.getByRole('button', { name: 'Dismiss notification', exact: true }).focus();
  await page.mouse.move(0, 0);
  await page.waitForTimeout(3500);
  await expect(notification).toBeVisible();
  await page.keyboard.press('Enter');
  await expect(notification).toHaveCount(0);
  await expect(page.locator('.canvas-workspace')).toBeFocused();
});

test('enlarged text and text spacing preserve command and inspector access', async ({ page }) => {
  await page.setViewportSize({ width: 720, height: 960 });
  await start(page);
  await page.addStyleTag({
    content: `html { font-size: 32px; } * { line-height: 1.5 !important; letter-spacing: .12em !important; word-spacing: .16em !important; } p { margin-bottom: 2em !important; }`
  });
  await page.getByRole('button', { name: 'Main menu', exact: true }).click();
  await page.getByRole('button', { name: 'Switch to Select', exact: true }).click();
  await expect(page.locator('.canvas-workspace')).toBeFocused();
  await page.getByRole('button', { name: 'Open Inspector', exact: true }).click();
  await page.getByRole('tab', { name: 'Properties', exact: true }).focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('tab', { name: 'Layers', exact: true })).toBeFocused();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('tab', { name: 'Export', exact: true })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('button', { name: 'Export PNG', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Open Inspector', exact: true })).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(720);
});

test('drawer feedback stays inside the dialog without covering its controls', async ({ page }) => {
  await start(page);
  await page.setViewportSize({ width: 320, height: 640 });
  await page.locator('.canvas-workspace').focus();
  await page.keyboard.press('Control+a');
  await page.keyboard.press('Control+c');
  await page.getByRole('button', { name: 'Open Inspector', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Inspector', exact: true });
  const notification = dialog.getByRole('group', { name: 'Notification', exact: true });
  await expect(notification).toBeVisible();
  await page.addStyleTag({ content: 'html { font-size: 32px; }' });
  const panel = (await page.locator('.inspector-panel').boundingBox())!;
  const feedback = (await notification.boundingBox())!;
  expect(feedback.y).toBeGreaterThanOrEqual(panel.y + panel.height - 1);
  await notification.getByRole('button', { name: 'Dismiss notification', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(notification).toHaveCount(0);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Open Inspector', exact: true })).toBeFocused();
});
