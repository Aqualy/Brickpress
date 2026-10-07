# Desktop validation

Validation on 7 October 2026. Brickpress's browser and Tauri applications share
one static frontend, the original catalog geometry, and version-1 projects.

## Completed checks

| Check                                | Result                                                                                         |
| ------------------------------------ | ---------------------------------------------------------------------------------------------- |
| Svelte / TypeScript analysis         | 0 errors, 0 warnings                                                                           |
| Unit tests                           | 86 passed                                                                                      |
| Catalog validation                   | All 41 pieces valid                                                                            |
| Browser production build             | Passed, static `build/` output                                                                 |
| Browser Playwright scenarios         | 36 passed                                                                                      |
| Windows native scenarios             | 17 application scenarios passed; the separate actual-dialog cancellation check passed on retry |
| macOS Apple Silicon native scenarios | 17 passed                                                                                      |
| macOS Intel native scenarios         | 17 passed                                                                                      |
| Linux x64 native scenarios           | 17 passed                                                                                      |
| Rust tests                           | 8 passed per target OS                                                                         |
| Rust formatting                      | Passed                                                                                         |
| Clippy                               | Passed with warnings denied, production and `desktop-e2e` features                             |
| npm production dependency audit      | 0 reported vulnerabilities (`npm audit --omit=dev`)                                            |

The hosted macOS/Linux results and browser checks are available in the
[test-installer workflow](https://github.com/Aqualy/Brickpress/actions/runs/37657752095).
Windows checks ran locally using WebView2. The optional real Windows dialog helper
had a transient UI Automation process-token error after the application scenarios
passed; an isolated rerun passed. The corrected narrow-layout scenario also
passed separately on Windows.

## Editor and native coverage

The toolbar, library and Inspector have fixed positions, with resizable sidebar
widths and responsive drawers. Preferences retain appearance and named print
presets, discarding unsupported fields without changing version-1 projects.
The library contains all 41 pieces. Printing-height compatibility is a placement
constraint, rather than a visual rendering mode.

Browser and native checks cover export previews, persistent export choices,
Print-Preview-only press settings, named custom print presets, ink-pass dragging
and keyboard reordering, focus restoration, and accessibility. Existing checks
cover placement, drag-to-fill, workspace marquee selection, multiple selections,
undo/redo, locks, printing-height compatibility, symmetry, and synchronized zoom.

The native embedded-driver suite checks Unicode saves, active-file overwrites,
Save As, cancellation, invalid projects, external changes, recovery, New and
Close/application Quit protection. It checks PNG/SVG transparency, project
exports that preserve unsaved status, tracing persistence, independent embossed
guides, middle-click sampling, keyboard placement, and readable bundled notices.
Native select-control tests dispatch their standard change event because the
embedded driver cannot reliably select HTML options; browser scenarios operate
those controls through Playwright.

Accessibility scans cover both modes and all Inspector tabs. Native screenshots
cover enlarged text at 320 CSS pixels and a 2,000-piece print-filter scenario.
Toolbar, mode-switch, Inspector-tab and footer labels are checked for clipping;
responsive controls wrap. Browser layout tests wait for responsive panels before
measuring their geometry, and native drawer tests wait for visible content.

Image decoding covers PNG, JPEG, WebP, GIF, AVIF and BMP, including image-element
fallback when `createImageBitmap` is unavailable. Ubuntu's tested WebKitGTK lacked
native AVIF decoding, so the application now bundles an AVIF decoder and stores
its lossless PNG result as a recoverable tracing guide. Browser and native tests
force the fallback, verify a decoded thumbnail and recovery, and preserve project
serialization. The codec's license and patent notices accompany the JavaScript
notices. The production CSP permits bundled WebAssembly compilation without
granting filesystem or shell access.

## Test installers

The [v1.0.1 test release](https://github.com/Aqualy/Brickpress/releases/tag/v1.0.1)
contains four installers built from `88ee3d19ddb765353f74722d68379a27c982892a`:

| Target              | Installer                         | Size in bytes |
| ------------------- | --------------------------------- | ------------: |
| macOS Apple Silicon | `Brickpress_1.0.1_aarch64.dmg`    |     9,308,458 |
| macOS Intel         | `Brickpress_1.0.1_x64.dmg`        |     9,475,566 |
| Linux x64           | `Brickpress_1.0.1_amd64.AppImage` |    84,519,416 |
| Debian/Ubuntu x64   | `Brickpress_1.0.1_amd64.deb`      |     6,264,524 |

All packaging jobs passed. Downloaded bytes matched each workflow checksum
before upload; the release includes a combined
[SHA256SUMS.txt](https://github.com/Aqualy/Brickpress/releases/download/v1.0.1/SHA256SUMS.txt).
Both macOS packaging logs confirm ad hoc signing with identity `-`. Linux
archive inspection verifies the x86-64 ELF executables, AppImage type 2, Debian
version/architecture, application icons, desktop entry and bundled project,
JavaScript and Rust license notices. Interactive installation remains a manual
check. Regular installers exclude the embedded test driver.

The previous [Windows v1.0.0 test release](https://github.com/Aqualy/Brickpress/releases/tag/v1.0.0)
remains separate. Its branded NSIS installer is 3,387,715 bytes, with SHA-256:

```text
33ff8cfc2249e9050eefcc77239ace97b8e32fe2fe61b3643351c4bc79d6a530
```

The preceding Windows install/uninstall smoke verified bundled notices, launch,
single-instance focus, recovery flush and clean close. The newly branded setup
and uninstaller icons were inspected directly from PE resources; their icon
layers match the supplied artwork. The branded installer pages have not been
checked interactively. User projects and application data are kept by uninstall.

## Remaining manual checks

- On macOS and Linux, test actual native pickers, interactive installation,
  security prompts, uninstall, and installed-app window-state restoration.
  Embedded-driver tests queue dialog outcomes and do not operate the OS picker UI.
- Exercise actual Save/Discard outcomes, read-only destinations, and force-quit
  recovery on each target OS. Windows actual-dialog cancellation is verified;
  controller failure/discard paths have unit coverage.
- Review the custom macOS DMG background and installed application icon on a Mac.
  The application uses a free ad hoc signature without publisher verification or
  notarization. Hands-on Gatekeeper behavior remains unverified.
- Track development-only npm advisories in the native test toolchain. The regular
  application excludes WebdriverIO, its embedded driver and debug test APIs.

Releases are published manually after validation. Mobile, automatic updates,
paid publisher signing/notarization, OS associations, recent files, multiple
windows and automatic browser-profile migration remain deferred. Browser and
native storage are separate. Standard hosted runners are free for the public
repository; no paid certificate or signing account is used.
