# Desktop validation

Local validation on Windows x64, 7 October 2026. The browser edition uses the
same production frontend as the native shell. Current artwork, catalog geometry,
version-1 documents, ink colors, dimensions, and paper presets are preserved.

## Completed checks

| Check                           | Result                                                                                |
| ------------------------------- | ------------------------------------------------------------------------------------- |
| Svelte / TypeScript analysis    | 0 errors, 0 warnings                                                                  |
| Unit tests                      | 86 passed                                                                             |
| Catalog validation              | All 41 pieces valid                                                                   |
| Browser production build        | Passed, static `build/` output                                                        |
| Browser Playwright scenarios    | All 35 passed against the fixed-layout production build                               |
| Windows native scenarios        | Preceding UI build: 18 passed, including Quit protection and real-dialog cancellation |
| Windows native clean close      | Passed; process exits normally after flushing version-1 recovery in isolated storage  |
| Rust tests                      | All 8 passed                                                                          |
| Rust formatting                 | Passed                                                                                |
| Clippy                          | Passed with warnings denied, production and `desktop-e2e` features                    |
| Windows production build        | Passed, x64 NSIS installer generated                                                  |
| Windows installer smoke         | Install, bundled notices, launch, single instance, clean close, and uninstall passed  |
| npm production dependency audit | 0 reported vulnerabilities (`npm audit --omit=dev`)                                   |

The latest UI update adds export previews, Print-Preview-only press controls,
custom print presets, and drag and keyboard ink-pass ordering. The toolbar,
library and Inspector have fixed positions, with resizable sidebar widths and
responsive drawers. Preferences migrate the
original browser string and desktop appearance record without changing version-1
projects. The library opens with all 41 pieces; printing-height constraints are
now named explicitly rather than presented as a visual mode.

Svelte analysis and formatting checks passed. Six new unit checks cover preference
validation/migration, saved press controls, supported preference fields and
undoable pass insertion. Four added browser scenarios cover export-renderer previews,
tab-persistent export choices, custom presets, pass dragging, focus and accessibility.
Native scenarios confirm export image decoding, print-preset recovery through
application data storage, and pass keyboard focus. The fixed-layout UI passed
Svelte/TypeScript analysis, all 86 unit tests, all 35 browser scenarios, catalog validation and a production
frontend build. macOS/Linux runs are planned next, using standard hosted runners
in the now-public repository.

The existing v1.0.0 GitHub release and preceding install/uninstall smoke result
remain unchanged. The latest frontend has not yet been packaged into installers.

The branding update uses the supplied transparent artwork without changing the
source bytes. The production frontend and Windows NSIS installer built again.
The generated ICO contains 16, 24, 32, 48, 64, and 256 px layers; the ICNS
container and merged Windows/macOS/Linux configurations passed local checks.
Icons extracted from the built Windows application and installer show the new
artwork. The generated NSIS script references the new header/sidebar bitmaps
and setup/uninstaller icons. The favicon is present in the production frontend.
The branded installer pages have not been checked in a running installer, and
the macOS DMG background still requires a macOS packaging/installation check.

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

## Previous v1.0.0 release installer

`artifacts/installers/Brickpress_1.0.0_x64-setup-branded.exe`

Size: 3,387,715 bytes. SHA-256:

```text
33ff8cfc2249e9050eefcc77239ace97b8e32fe2fe61b3643351c4bc79d6a530
```

This is an unsigned test build. It contains JavaScript and Rust dependency
notices as resources; the editor also provides a readable notices dialog.
The preceding installer smoke check used `artifacts/installed/Brickpress` and
removed it using its uninstaller. The newly branded installer has been built
and inspected but has not been installed. User projects and application data
are kept by the uninstaller.

The drag-to-fill update rebuilt the Windows application and installer. All
Windows icon groups in both executables match the supplied artwork pixel for
pixel when loaded directly from their PE resources, bypassing Explorer's cached
preview. Both the setup and uninstaller explicitly use the custom ICO and
header bitmap. A copy at
`artifacts/installers/Brickpress_1.0.0_x64-setup-branded.exe` has a fresh filename
for downloading; its checksum is in the same directory.

The 80 unit tests include continuous-grid traversal, clipped paths outside the
paper, failed placement groups, immediate piece display, collision/footprint
rules, symmetry, locks, physical compatibility, the 10,000-piece limit, stroke
cancellation, redo preservation, and one undo transaction per stroke. Four new
browser scenarios exercise sparse and diagonal pointer movement, retracing with
overlap enabled, rotated pieces around obstacles, paper edges, Print Preview,
Escape, actual capture release, and pointer cancellation. The placement cursor
continues to support arrows and Enter for keyboard placement. All 31 browser
scenarios passed against the rebuilt production frontend, including the
accessibility, enlarged-text, export, preset, tracing and 2,000-piece scenarios.

## Outstanding platform checks

- macOS Apple Silicon/Intel and Linux builds, native smoke runs, actual dialogs,
  and installation are configured in GitHub Actions but have not been executed
  from this Windows workspace. Hosted tests are held pending confirmation that
  paid Actions usage is disabled; the source push skips automatic CI.
- The zero-budget macOS configuration has a free ad hoc signature. Its merged
  Tauri configuration was checked locally; signing and Gatekeeper behavior still
  require a macOS build and installation check. The installer checksum generator
  matched PowerShell's SHA-256 result for the existing Windows installer.
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
and automatic browser-profile migration remain deferred. Paid publisher signing
is not part of the zero-budget plan; macOS ad hoc signing does not verify a
publisher identity or provide notarization.
