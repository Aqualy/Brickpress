import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { createDocument, AUTOSAVE_KEY } from '../src/lib/persistence/document';
import { pieces } from '../src/lib/catalog/catalog';
import type { PressDocument } from '../src/lib/types/document';
import { capturePreset, PRESETS_KEY } from '../src/lib/persistence/presets';

async function start(page: Page, doc = createDocument()) {
  doc.board = { width: 8, height: 8 };
  await page.addInitScript(({ key, doc }) => localStorage.setItem(key, JSON.stringify(doc)), {
    key: AUTOSAVE_KEY,
    doc
  });
  await page.goto('/');
  await page.getByRole('button', { name: '3D view', exact: true }).click();
  await expect(page.locator('.three-canvas')).toHaveAttribute('data-ready', 'true', {
    timeout: 30000
  });
}
async function state(page: Page): Promise<PressDocument> {
  await expect(page.locator('.local-save')).toHaveText('Saved on this device');
  return page.evaluate((key) => JSON.parse(localStorage.getItem(key)!), AUTOSAVE_KEY);
}
async function top(page: Page) {
  await page.getByRole('button', { name: '3D camera controls', exact: true }).click();
  await page.getByRole('button', { name: 'Top', exact: true }).click();
  await page.keyboard.press('Escape');
  await page.locator('.zoom-value').click();
}
async function point(page: Page, x: number, y: number) {
  const b = (await page.locator('.three-canvas').boundingBox())!;
  return { x: b.x + b.width / 2 + (x - 4) * 32, y: b.y + b.height / 2 + (y - 4) * 32 };
}
async function drag(
  page: Page,
  from: { x: number; y: number },
  to: { x: number; y: number },
  button: 'left' | 'right' | 'middle' = 'left'
) {
  await page.mouse.move(from.x, from.y);
  await page.mouse.down({ button });
  await page.mouse.move(to.x, to.y, { steps: 5 });
  await page.mouse.up({ button });
}

test('3D placement, dragging and undo use the same document as Design', async ({ page }) => {
  await start(page);
  await top(page);
  await page.getByRole('button', { name: 'Place Tile 1×1, 3070', exact: true }).click();
  const p = await point(page, 2.5, 2.5);
  await page.mouse.click(p.x, p.y);
  expect((await state(page)).passes[0].pieces).toHaveLength(1);
  await page.getByRole('button', { name: 'Select tool', exact: true }).click();
  await drag(page, p, await point(page, 4.5, 3.5));
  expect((await state(page)).passes[0].pieces[0]).toMatchObject({ x: 4, y: 3 });
  await page.keyboard.press('Control+z');
  expect((await state(page)).passes[0].pieces[0]).toMatchObject({ x: 2, y: 2 });
  await page.getByRole('button', { name: 'Design', exact: true }).click();
  await expect(page.locator('.artboard [data-uid]')).toHaveCount(1);
  await page.locator('.canvas-workspace').focus();
  await page.keyboard.press('Control+Shift+z');
  expect((await state(page)).passes[0].pieces[0]).toMatchObject({ x: 4, y: 3 });
  await page.getByRole('button', { name: '3D view', exact: true }).click();
  await expect(page.locator('.three-canvas')).toHaveAttribute('data-ready', 'true');
  await page.locator('.canvas-workspace').focus();
  await page.keyboard.press('ArrowRight');
  expect((await state(page)).passes[0].pieces[0].x).toBe(5);
});

test('3D drag-to-fill, cancellation, marquee and middle-click sampling', async ({ page }) => {
  await start(page);
  await top(page);
  await page.getByRole('button', { name: 'Place Tile 1×1, 3070', exact: true }).click();
  await drag(page, await point(page, 1.5, 2.5), await point(page, 5.5, 2.5));
  expect((await state(page)).passes[0].pieces).toHaveLength(5);
  await page.keyboard.press('Control+z');
  expect((await state(page)).passes[0].pieces).toHaveLength(0);
  await page.keyboard.press('Control+Shift+z');
  const begin = await point(page, 1.5, 4.5);
  await page.mouse.move(begin.x, begin.y);
  await page.mouse.down();
  const end = await point(page, 5.5, 4.5);
  await page.mouse.move(end.x, end.y);
  await page.keyboard.press('Escape');
  await page.mouse.up();
  expect((await state(page)).passes[0].pieces).toHaveLength(5);
  await page.getByRole('button', { name: 'Select tool', exact: true }).click();
  await drag(page, await point(page, -0.5, 1.5), await point(page, 6.5, 3.5));
  await expect(page.locator('.three-status')).toContainText('5 selected');
  await page.keyboard.press('ArrowDown');
  expect((await state(page)).passes[0].pieces.every((p) => p.y === 3)).toBe(true);
  await page.getByRole('button', { name: 'Place tool', exact: true }).click();
  const sample = await point(page, 1.5, 3.5);
  await page.mouse.click(sample.x, sample.y, { button: 'middle' });
  await expect(page.getByRole('status').filter({ hasText: 'Picked Tile 1×1' })).toBeVisible();
  await page.locator('.canvas-workspace').focus();
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  expect((await state(page)).passes[0].pieces).toHaveLength(6);
});

test('3D respects curved holes, collision rejection and locked pieces', async ({ page }) => {
  const doc = createDocument();
  doc.passes[0].pieces = [{ uid: 'ring', pieceId: '27925', x: 2, y: 2, rotation: 0, seed: 1 }];
  doc.passes.push({
    ...doc.passes[0],
    id: 'locked',
    name: 'Locked',
    locked: true,
    pieces: [{ uid: 'locked-piece', pieceId: '3070', x: 6, y: 6, rotation: 0, seed: 1 }]
  });
  await start(page, doc);
  await top(page);
  await page
    .getByRole('button', { name: 'Place Tile Round 1×1 Quarter, 25269', exact: true })
    .click();
  const p = await point(page, 2.2, 2.2);
  await page.mouse.click(p.x, p.y);
  expect((await state(page)).passes[0].pieces).toHaveLength(2);
  await page.mouse.click(p.x, p.y);
  expect((await state(page)).passes[0].pieces).toHaveLength(2);
  await page.getByRole('button', { name: 'Select tool', exact: true }).click();
  await drag(page, await point(page, 6.5, 6.5), await point(page, 5.5, 6.5));
  expect((await state(page)).passes.find((pass) => pass.id === 'locked')!.pieces[0]).toMatchObject({
    x: 6,
    y: 6
  });
  await page.getByRole('button', { name: 'Place tool', exact: true }).click();
  const sample = await point(page, 6.5, 6.5);
  await page.mouse.click(sample.x, sample.y, { button: 'middle' });
  await expect(page.getByRole('status').filter({ hasText: 'Picked Tile 1×1' })).toBeVisible();
});

test('3D camera controls preserve serialization, focus and accessible narrow layout', async ({
  page
}) => {
  await start(page);
  const before = await state(page);
  const initialZoom = await page.locator('.zoom-value').textContent();
  const b = (await page.locator('.three-canvas').boundingBox())!;
  await drag(
    page,
    { x: b.x + b.width / 2, y: b.y + b.height / 2 },
    { x: b.x + b.width / 2 + 70, y: b.y + b.height / 2 + 35 },
    'right'
  );
  await page.locator('.canvas-workspace').focus();
  await page.keyboard.press('Alt+ArrowRight');
  await expect(page.locator('.zoom-value')).toHaveText(initialZoom!);
  await page.getByRole('button', { name: 'Canvas zoom in', exact: true }).click();
  expect(await state(page)).toEqual(before);
  await page.getByRole('button', { name: '3D camera controls', exact: true }).click();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: '3D camera controls', exact: true })).toBeFocused();
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'])
        .analyze()
    ).violations
  ).toEqual([]);
  await page.screenshot({ path: 'artifacts/three/desktop.png' });
  await page.setViewportSize({ width: 320, height: 640 });
  await expect(page.locator('.three-canvas')).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true
  );
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'])
        .analyze()
    ).violations
  ).toEqual([]);
  await page.screenshot({ path: 'artifacts/three/narrow.png' });
  await page.getByRole('button', { name: 'Print preview', exact: true }).click();
  await expect(page.locator('.artboard')).toBeVisible();
  expect(await state(page)).toEqual(before);
});

test('3D fit holds projection scale through orbit and pan, and still reframes on demand', async ({
  page
}) => {
  await start(page);
  const result = await page.evaluate(async (autosaveKey) => {
    const modulePath = '/src/lib/three/scene.ts';
    const { BoardScene } = await import(/* @vite-ignore */ modulePath);
    const canvas = document.createElement('canvas');
    let state = { azimuth: Math.PI / 4, elevation: Math.PI / 3, target: [0, 0, 0] };
    let zoom = 0;
    const errors: string[] = [];
    const scene = new BoardScene(
      canvas,
      state,
      (camera: typeof state, value: number) => {
        state = camera;
        zoom = value;
      },
      (error: string) => errors.push(error)
    );
    try {
      const doc = JSON.parse(localStorage.getItem(autosaveKey)!);
      doc.board = { width: 8, height: 16 };
      scene.setProjection('orthographic');
      scene.syncDocument(doc);
      scene.resize(900, 700);
      scene.setZoom('fit');
      // A unit segment parallel to the camera's horizontal axis should keep
      // its pixel length at every angle, regardless of natural foreshortening.
      const span = () => {
        const a = scene.project(4, 8);
        const b = scene.project(4 + Math.cos(state.azimuth), 8 - Math.sin(state.azimuth));
        return Math.hypot(b.x - a.x, b.y - a.y);
      };
      const initial = { zoom, span: span() };
      const rotations = [];
      for (const [dx, dy] of [
        [140, -80],
        [-290, 140],
        [600, -200],
        [-800, 300]
      ]) {
        scene.orbit(dx, dy);
        rotations.push({ zoom, span: span() });
      }
      scene.pan(60, -30);
      const panned = { zoom, span: span() };
      scene.setZoom('fit');
      const refitted = { zoom, span: span(), target: state.target };
      scene.setZoom(150);
      scene.orbit(300, -40);
      const explicit = { zoom, span: span() };
      scene.resize(600, 500);
      const resized = { zoom, span: span() };
      return { initial, rotations, panned, refitted, explicit, resized, errors };
    } finally {
      scene.dispose();
    }
  }, AUTOSAVE_KEY);
  expect(result.errors).toEqual([]);
  for (const sample of [...result.rotations, result.panned]) {
    expect(sample.zoom).toBe(result.initial.zoom);
    expect(sample.span).toBeCloseTo(result.initial.span, 8);
  }
  expect(result.refitted.target).toEqual([0, 0, 0]);
  expect(result.refitted.span).not.toBeCloseTo(result.initial.span, 2);
  expect(result.explicit.zoom).toBe(150);
  expect(result.explicit.span).toBeCloseTo(48, 8);
  expect(result.resized.zoom).toBe(150);
  expect(result.resized.span).toBeCloseTo(result.explicit.span, 8);
});

test('linear perspective is default, projection switches are accessible and stay outside projects', async ({
  page
}) => {
  await start(page);
  const before = await state(page);
  const canvas = page.locator('.three-canvas');
  await expect(canvas).toHaveAttribute('data-projection', 'perspective');
  await page.getByRole('button', { name: '3D camera controls', exact: true }).click();
  const perspective = page.getByRole('button', { name: 'Linear perspective', exact: true });
  const orthographic = page.getByRole('button', { name: 'Orthographic', exact: true });
  await expect(perspective).toHaveAttribute('aria-pressed', 'true');
  await orthographic.focus();
  await page.keyboard.press('Enter');
  await expect(canvas).toHaveAttribute('data-projection', 'orthographic');
  await expect(orthographic).toHaveAttribute('aria-pressed', 'true');
  await expect(perspective).toHaveAttribute('aria-pressed', 'false');
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'])
        .analyze()
    ).violations
  ).toEqual([]);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: '3D camera controls', exact: true })).toBeFocused();
  await page.getByRole('button', { name: 'Design', exact: true }).click();
  await page.getByRole('button', { name: '3D view', exact: true }).click();
  await expect(canvas).toHaveAttribute('data-ready', 'true');
  await expect(canvas).toHaveAttribute('data-projection', 'orthographic');
  await page.getByRole('button', { name: '3D camera controls', exact: true }).click();
  await perspective.click();
  await page.getByRole('button', { name: 'Reset camera', exact: true }).click();
  await expect(canvas).toHaveAttribute('data-projection', 'perspective');
  await expect(perspective).toHaveAttribute('aria-pressed', 'true');
  await page.keyboard.press('Escape');
  const zoom = await page.locator('.zoom-value').textContent();
  await page.locator('.canvas-workspace').focus();
  await page.keyboard.press('Alt+ArrowDown');
  await expect(page.locator('.zoom-value')).toHaveText(zoom!);
  expect(await state(page)).toEqual(before);
  await page.setViewportSize({ width: 320, height: 640 });
  await page.getByRole('button', { name: '3D camera controls', exact: true }).click();
  await expect(perspective).toBeVisible();
  await expect(orthographic).toBeVisible();
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'])
        .analyze()
    ).violations
  ).toEqual([]);
  await page.keyboard.press('Escape');
});

test('3D accepts preset drops and keeps tracing outside the project', async ({ page }) => {
  const source = createDocument();
  source.passes[0].pieces = [{ uid: 'source', pieceId: '3069', x: 0, y: 0, rotation: 0, seed: 1 }];
  const preset = capturePreset(source, 'Pair');
  await page.addInitScript(
    ({ key, preset }) => localStorage.setItem(key, JSON.stringify([preset])),
    { key: PRESETS_KEY, preset }
  );
  await start(page);
  await top(page);
  const p = await point(page, 3.5, 3.5);
  await page.locator('.three-canvas').evaluate(
    (canvas, { p, id }) => {
      const dataTransfer = new DataTransfer();
      dataTransfer.setData('application/x-brickpress-preset', id);
      canvas.dispatchEvent(
        new DragEvent('drop', { bubbles: true, clientX: p.x, clientY: p.y, dataTransfer })
      );
    },
    { p, id: preset.id }
  );
  expect((await state(page)).passes.flatMap((p) => p.pieces)).toHaveLength(1);
  const before = await state(page);
  await page.getByRole('button', { name: 'Grid settings', exact: true }).click();
  await page
    .getByLabel('Upload tracing image file')
    .setInputFiles('tests/native/fixtures/guide.png');
  await expect(
    page.getByRole('button', { name: 'Remove tracing image', exact: true })
  ).toBeVisible();
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Design', exact: true }).click();
  await expect(page.locator('[data-editor-guide="tracing"]')).toBeVisible();
  expect(await state(page)).toEqual(before);
  await page.getByRole('button', { name: '3D view', exact: true }).click();
  await expect(page.locator('.three-canvas')).toHaveAttribute('data-ready', 'true');
  await page.locator('.three-canvas').dispatchEvent('webglcontextlost');
  await expect(page.getByRole('alert')).toContainText('graphics context was lost');
  await page.getByRole('button', { name: 'Return to Design', exact: true }).click();
  expect(await state(page)).toEqual(before);
});

test('3D falls back safely when WebGL is unavailable', async ({ page }) => {
  await page.addInitScript(() => {
    const original = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (
      this: HTMLCanvasElement,
      type: string,
      ...args: unknown[]
    ) {
      if (type === 'webgl2' || type === 'webgl') return null;
      return original.apply(this, [type, ...args] as Parameters<typeof original>);
    } as typeof original;
  });
  await page.goto('/');
  await page.getByRole('button', { name: '3D view', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('3D graphics are unavailable');
  await page.getByRole('button', { name: 'Return to Design', exact: true }).click();
  await expect(page.locator('.artboard')).toBeVisible();
});

test('all catalog meshes preserve holes and fit their footprints; instanced scenes stay idle', async ({
  page
}) => {
  await page.goto('/');
  const results = await page.evaluate(
    async ({ doc, catalog }) => {
      const path = '/src/lib/three/scene.ts';
      const { BoardScene } = await import(/* @vite-ignore */ path);
      const geometryPath = '/src/lib/three/geometry.ts';
      const { makePieceGeometry } = await import(/* @vite-ignore */ geometryPath);
      const canvas = document.createElement('canvas');
      canvas.style.cssText = 'position:fixed;inset:0;width:800px;height:800px';
      document.body.append(canvas);
      const scene = new BoardScene(
        canvas,
        null,
        () => {},
        (message: string) => {
          throw new Error(message);
        }
      );
      const dimensions = catalog.map((p) => {
        const g = makePieceGeometry({ pieceId: p.id, rotation: 0, uid: 'p', x: 0, y: 0, seed: 1 });
        const b = g.boundingBox;
        const result = {
          id: p.id,
          width: b.max.x - b.min.x,
          height: b.max.z - b.min.z,
          expected: p.footprint
        };
        g.dispose();
        return result;
      });
      doc.board = { width: 128, height: 128 };
      doc.passes[0].pieces = Array.from({ length: 2000 }, (_, i) => ({
        uid: `p${i}`,
        pieceId: '3070',
        x: i % 100,
        y: Math.floor(i / 100),
        rotation: 0,
        seed: 1
      }));
      scene.resize(800, 800);
      scene.syncDocument(doc);
      scene.cameraAction('top');
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const first = scene.diagnostics();
      await new Promise((r) => setTimeout(r, 150));
      const idle = scene.diagnostics();
      const pick = scene.project(0.5, 0.5);
      const uid = scene.pick(pick.x, pick.y);
      // The round 1x1 quarter's empty corner must not intercept picking.
      doc.board = { width: 8, height: 8 };
      doc.passes[0].pieces = [
        { uid: 'quarter', pieceId: '25269', x: 2, y: 2, rotation: 0, seed: 1 }
      ];
      scene.syncDocument(doc);
      scene.cameraAction('top');
      const corners = [
        [2.1, 2.1],
        [2.9, 2.1],
        [2.1, 2.9],
        [2.9, 2.9]
      ].map(([x, y]) => {
        const p = scene.project(x, y);
        return scene.pick(p.x, p.y);
      });
      scene.dispose();
      canvas.remove();
      return { dimensions, first, idle, uid, corners };
    },
    { doc: createDocument(), catalog: pieces }
  );
  for (const p of results.dimensions) {
    expect(p.width, p.id).toBeGreaterThan(0);
    expect(p.width, p.id).toBeLessThanOrEqual(p.expected.widthStuds + 0.001);
    expect(p.height, p.id).toBeLessThanOrEqual(p.expected.heightStuds + 0.001);
  }
  expect(results.first.draws).toBeLessThan(20);
  expect(results.idle.frames).toBe(results.first.frames);
  expect(results.uid).toBe('p0');
  expect(results.corners).toContain(null);
  expect(results.corners).toContain('quarter');
});
