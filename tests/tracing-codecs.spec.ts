import { test, expect } from '@playwright/test';
import { createDocument, AUTOSAVE_KEY } from '../src/lib/persistence/document';

test('AVIF fallback stays local, produces a renderable PNG and recovers without changing the project', async ({
  page
}) => {
  await page.addInitScript(({ key, doc }) => localStorage.setItem(key, JSON.stringify(doc)), {
    key: AUTOSAVE_KEY,
    doc: createDocument(true)
  });
  await page.goto('/');
  await expect(page.locator('.editor-app')).toHaveAttribute('data-ready', 'true');
  await page.getByRole('button', { name: 'Dismiss editor hint' }).click();
  const before = await page.evaluate((key) => localStorage.getItem(key), AUTOSAVE_KEY);
  await page.evaluate(() => {
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
            if (avifUrls.has(value)) queueMicrotask(() => image.dispatchEvent(new Event('error')));
            else src.set!.call(image, value);
          },
          get() {
            return src.get!.call(image);
          }
        });
        return image;
      }
    });
    window.createImageBitmap = async () => {
      throw new Error('Codec unavailable');
    };
  });
  await page.getByRole('button', { name: 'Grid settings', exact: true }).click();
  await page
    .getByLabel('Upload tracing image file')
    .setInputFiles('tests/native/fixtures/guide.avif');
  const guide = page.locator('[data-editor-guide="tracing"]');
  await expect(guide).toBeVisible();
  await expect
    .poll(() =>
      page.evaluate(async () => {
        const image = document.querySelector('[data-editor-guide="tracing"]')!;
        const asset = await (await fetch(image.getAttribute('href')!)).blob();
        return asset.type;
      })
    )
    .toBe('image/png');
  await expect
    .poll(() => page.evaluate((key) => localStorage.getItem(key), AUTOSAVE_KEY))
    .toBe(before);
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          new Promise<string>((resolve, reject) => {
            const request = indexedDB.open('brickpress-guides', 1);
            request.onsuccess = () => {
              const db = request.result;
              const record = db.transaction('tracing').objectStore('tracing').get('current');
              record.onsuccess = () => {
                resolve(record.result?.asset.type ?? '');
                db.close();
              };
              record.onerror = () => {
                reject(record.error);
                db.close();
              };
            };
            request.onerror = () => reject(request.error);
          })
      )
    )
    .toBe('image/png');
  await page.keyboard.press('Escape');
  await page.reload();
  await expect(page.locator('.editor-app')).toHaveAttribute('data-ready', 'true');
  await expect(guide).toBeVisible();
  await page.getByRole('button', { name: 'Grid settings', exact: true }).click();
  const thumbnail = page.locator('.trace-summary img');
  await expect
    .poll(() => thumbnail.evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth > 0))
    .toBe(true);
});
