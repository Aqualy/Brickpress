import { resolve } from 'node:path';
import { mkdirSync } from 'node:fs';

const data = resolve('artifacts/native/data');
mkdirSync(data, { recursive: true });
process.env.BRICKPRESS_TEST_DATA_DIR = data;
process.env.TAURI_WEBDRIVER_PORT = '4445';

export const config = {
  runner: 'local',
  specs: ['./tests/native/desktop.spec.ts'],
  maxInstances: 1,
  logLevel: 'warn',
  outputDir: './artifacts/native/logs',
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: { timeout: 90000 },
  waitforTimeout: 15000,
  connectionRetryTimeout: 90000,
  connectionRetryCount: 1,
  services: [
    [
      '@wdio/tauri-service',
      {
        driverProvider: 'embedded',
        embeddedPort: 4445,
        startTimeout: 90000,
        captureBackendLogs: true,
        captureFrontendLogs: true,
        env: { BRICKPRESS_TEST_DATA_DIR: data, TAURI_WEBDRIVER_PORT: '4445' }
      }
    ]
  ],
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: resolve(
          `src-tauri/target/debug/brickpress${process.platform === 'win32' ? '.exe' : ''}`
        )
      }
    }
  ],
  async afterTest(_test: unknown, _context: unknown, result: { passed: boolean }) {
    if (!result.passed) {
      const { browser } = await import('@wdio/globals');
      await browser.saveScreenshot(`artifacts/native/failure-${Date.now()}.png`);
    }
  }
};
