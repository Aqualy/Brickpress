# Desktop validation

Local validation on Windows x64, 7 October 2026. The browser edition uses the
same production frontend as the native shell. Current artwork, catalog geometry,
version-1 documents, ink colors, dimensions, and paper presets are preserved.

## Completed checks

| Check                           | Result                                                                               |
| ------------------------------- | ------------------------------------------------------------------------------------ |
| Svelte / TypeScript analysis    | 0 errors, 0 warnings                                                                 |
| Unit tests                      | 69 passed                                                                            |
| Catalog validation              | All 41 pieces valid                                                                  |
| Browser production build        | Passed, static `build/` output                                                       |
| Browser Playwright scenarios    | All 27 passed against production preview                                             |
| Windows native scenarios        | All 16 passed, including application Quit protection and the real-dialog check       |
| Windows native clean close      | Passed; process exits normally after flushing version-1 recovery in isolated storage |
| Rust tests                      | All 8 passed                                                                         |
| Rust formatting                 | Passed                                                                               |
| Clippy                          | Passed with warnings denied, production and `desktop-e2e` features                   |
| Windows production build        | Passed, x64 NSIS installer generated                                                 |
| Windows installer smoke         | Install, bundled notices, launch, single instance, clean close, and uninstall passed |
| npm production dependency audit | 0 reported vulnerabilities (`npm audit --omit=dev`)                                  |

The latest editor update adds workspace marquee selection and consolidates Export,
size, zoom, selection actions, and save status. The 27 browser scenarios include
outside-artboard dragging in both directions, additive selection, hidden/locked
passes, zoom/pan alignment, cancellation, and desktop/320 px control layouts.
Svelte analysis, all 69 unit tests, the production build, and formatting checks
passed again. Native smoke and installer behavior results above are from the
preceding Tauri integration; native selectors have been updated for these UI
changes, but the native suite has not been rerun for this update.

The native embedded-driver scenarios check Unicode saves, overwriting the active
file, Save As, cancellation, invalid projects, external changes, recovery, New
and close/application-Quit protection, keyboard placement, middle-click sampling, synchronized
zoom, inspector tabs, independent embossed guides, tracing persistence, PNG/SVG
transparency, and project exports that leave unsaved status intact. Accessibility
scans cover both modes and all inspector tabs. Native screenshots cover enlarged
text at 320 CSS pixels and the 2,000-piece print-filter scenario. Enlarged toolbar,
mode-switch, inspector-tab, and footer labels are checked for clipping; responsive
controls wrap, and status messages track the footer's height. Decoder tests
cover PNG, JPEG, WebP, GIF, AVIF, and BMP, plus image-element decoding when
`createImageBitmap` is unavailable. The native notices dialog is also checked.

The separate real Windows dialog test cancels actual Open/Save dialogs and the
Save / Discard / Cancel close confirmation using PowerShell 7 UI Automation.
The default tests use scoped dialog queues to make file/controller outcomes
repeatable. The test driver does not implement HTML option selection reliably;
that control test sends its standard change event. Browser tests exercise native
select controls through Playwright.

## Windows test installer

`src-tauri/target/release/bundle/nsis/Brickpress_1.0.0_x64-setup.exe`

Size: 3,118,548 bytes. SHA-256:

```text
bea5466de8544e1bff1ec08120a058d731a9a76fde4a20cf469b7570aae0c2d5
```

This is an unsigned test build. It contains JavaScript and Rust dependency
notices as resources; the editor also provides a readable notices dialog.
The isolated installation used `artifacts/installed/Brickpress` and was removed
by its uninstaller after testing. User projects and application data are kept.

## Outstanding platform checks

- macOS Apple Silicon/Intel and Linux builds, native smoke runs, actual dialogs,
  and installation are configured in GitHub Actions but have not been executed
  from this Windows workspace. The workflows are local changes and must be
  pushed before they can run.
- Real dialog Save/Discard outcomes, read-only destinations, interactive window
  position/size/maximized-state restoration, and unsigned OS prompts need the
  documented manual checks on each platform. Cancellation has been checked on
  Windows; file/controller failure and discard paths have unit coverage.
- Webview rendering and tracing codecs can vary by OS. Windows native checks do
  not establish macOS/Linux visual or decoder behavior.
- The full npm audit still reports development-only transitive advisories in the
  new native testing toolchain (including `braces` and `extract-zip`). Available
  compatible fixes were applied. The installed application does not contain
  WebdriverIO, its embedded driver, or the debug-only test APIs. Track upstream
  fixes before expanding automated testing to untrusted inputs.

No release is published automatically. Private installer artifacts require the
manually triggered workflow. Mobile, automatic updates, signing/notarization,
public distribution, OS associations, recent files, multiple document windows,
and automatic browser-profile migration remain deferred.
