# Desktop validation

## Brickpress 1.1.0 — 13 October 2026

The [all-platform workflow](https://github.com/Aqualy/Brickpress/actions/runs/38329862595)
tests source commit `37e5144af794eae0f3b24724f768d090fb193255`.

| Check                                      | Result                                                   |
| ------------------------------------------ | -------------------------------------------------------- |
| Svelte / TypeScript                        | 0 errors, 0 warnings                                     |
| Unit tests                                 | 95 passed in 10 files                                    |
| Catalog                                    | All 41 pieces valid                                      |
| Static production build                    | Passed                                                   |
| Browser scenarios                          | 52 passed                                                |
| Windows native scenarios                   | 18 passed; WebGL 2 rendering and editing verified        |
| Apple Silicon native scenarios             | 18 passed; WebGL 2 rendering and editing verified        |
| Linux native scenarios                     | 18 passed; WebGL 2 rendering and editing verified        |
| Intel macOS native scenarios               | 18 passed; unavailable-graphics fallback verified        |
| Rust tests                                 | 8 passed per target OS                                   |
| Rust formatting and Clippy                 | Passed; warnings denied for production and test features |
| Windows native close                       | Clean exit and version-1 recovery flush passed           |
| Public repository hygiene and secret scans | Passed for complete Git history                          |
| npm production audit                       | 0 reported vulnerabilities                               |

The native graphics probe independently records a context's version and lost
state. The hosted Intel Mac returned an already-lost context with a null version.
Its tests therefore check the accessible no-graphics message, artwork preservation,
and return to Design, including a 2,000-piece document. It did not verify 3D
rendering on Intel hardware. Windows, Apple Silicon and Linux verified shared 3D
editing, both projections, stable orbit zoom, and the 2,000-piece renderer.
Capable devices still fail the test if the application's renderer fails.

Browser and native accessibility scans cover Design, Print Preview, and 3D
(the fallback on the Intel runner), across every Inspector tab. The suite also
covers keyboard controls, 320 px layouts, enlarged text, tracing formats, PNG/SVG
transparency, file handling, recovery and presets. Embedded-driver runs use
isolated storage and queued dialog responses. The optional real Windows picker
test is skipped in hosted CI; interactive installation, upgrades and actual
native pickers remain manual checks on each OS. Earlier Windows picker testing
below applies to the earlier release, not a new hands-on verification of 1.1.0.

Production installers exclude the embedded test driver. Windows is unsigned;
macOS uses free ad hoc signing without notarization. Physical Intel Mac 3D
rendering, installed-app launch, OS security prompts and interactive uninstall
still need hands-on testing. Existing development-only npm alerts and Linux GTK
dependency warnings remain documented in [SECURITY.md](../SECURITY.md).

### Test installers

The [v1.1.0 test release](https://github.com/Aqualy/Brickpress/releases/tag/v1.1.0)
contains five installers built from the source commit above. Intel DMG creation
succeeded on a packaging-job retry; the same tested source was used throughout.

| Target              | Installer                         | Size in bytes |
| ------------------- | --------------------------------- | ------------: |
| Windows x64         | `Brickpress_1.1.0_x64-setup.exe`  |     3,865,496 |
| macOS Apple Silicon | `Brickpress_1.1.0_aarch64.dmg`    |     9,460,235 |
| macOS Intel         | `Brickpress_1.1.0_x64.dmg`        |     9,620,846 |
| Linux x64           | `Brickpress_1.1.0_amd64.AppImage` |    84,666,872 |
| Debian/Ubuntu x64   | `Brickpress_1.1.0_amd64.deb`      |     6,411,488 |

Downloaded bytes matched all four platform checksum manifests. The release
includes a combined [SHA256SUMS.txt](https://github.com/Aqualy/Brickpress/releases/download/v1.1.0/SHA256SUMS.txt).
Windows metadata reports version 1.1.0; CI verified both the application and NSIS
installer's embedded icons against the supplied artwork. Windows and both Mac
archives contain the application icon and all four license/notice files. Mac
bundle metadata reports version 1.1.0 and `io.github.aqualy.brickpress`; both
packaging logs confirm ad hoc signing. Linux inspection verifies x64 ELF
executables, AppImage type 2, Debian version/architecture, icons, desktop entry
and the four notice files. These inspections do not install or launch packages.

## Historical validation — Brickpress 1.0.1

Windows and repository validation on 8 October 2026; macOS/Linux and browser
validation on 7 October 2026. Brickpress's browser and Tauri applications share
one static frontend, the original catalog geometry, and version-1 projects.

### Completed checks

| Check                                | Result                                                                  |
| ------------------------------------ | ----------------------------------------------------------------------- |
| Svelte / TypeScript analysis         | 0 errors, 0 warnings                                                    |
| Unit tests                           | 86 passed                                                               |
| Catalog validation                   | All 41 pieces valid                                                     |
| Browser production build             | Passed, static `build/` output                                          |
| Browser Playwright scenarios         | 36 passed                                                               |
| Windows native scenarios             | 18 passed, including actual Open/Save/Close dialog cancellation         |
| macOS Apple Silicon native scenarios | 17 passed                                                               |
| macOS Intel native scenarios         | 17 passed                                                               |
| Linux x64 native scenarios           | 17 passed                                                               |
| Rust tests                           | 8 passed per target OS                                                  |
| Rust formatting                      | Passed                                                                  |
| Clippy                               | Passed with warnings denied, production and `desktop-e2e` features      |
| npm production dependency audit      | 0 reported vulnerabilities (`npm audit --omit=dev`)                     |
| Rust dependency audit                | 0 vulnerability-category advisories; two open Linux dependency warnings |
| Public repository audit              | Full Git history and current-source scans passed                        |

The hosted macOS/Linux results and browser checks are available in the
[test-installer workflow](https://github.com/Aqualy/Brickpress/actions/runs/37657752095).
Windows checks ran locally using WebView2. The complete native suite, including
the actual-dialog cancellation helper and narrow-layout scenario, passed on
8 October. Clippy and the hosted/browser results above are from 7 October; the
application source and dependency lockfiles did not change between those builds.
See [the repository audit](repository-audit.md) and [security policy](../SECURITY.md)
for scan scope and the open dependency advisories.

### Editor and native coverage

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

### Test installers

The [v1.0.1 test release](https://github.com/Aqualy/Brickpress/releases/tag/v1.0.1)
contains five installers. The macOS/Linux packages were built from
`88ee3d19ddb765353f74722d68379a27c982892a`; Windows was built from
`2499a3e94c3aadc1163c6833c8029e1c18f216ee`. Their application source and lockfiles
are identical; the later commit prepares the public repository and documentation.

| Target              | Installer                         | Size in bytes |
| ------------------- | --------------------------------- | ------------: |
| Windows x64         | `Brickpress_1.0.1_x64-setup.exe`  |     3,713,876 |
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

The Windows v1.0.1 NSIS installer has SHA-256:

```text
8061904c5ac852463f13f711b5acbc818650125cbd191406ccb056cb019eeddc
```

Windows application and installer PE resources report version 1.0.1 and their
icon layers match the supplied artwork. The generated NSIS script includes the
custom sidebar/header and all four license/notice documents. A regular production
build produced the release artifact without the embedded test driver.

The preceding v1.0.0 install/uninstall smoke verified bundled notices, launch,
single-instance focus, recovery flush and clean close. Interactive installation
or upgrade of the new 1.0.1 installer remains unverified; the existing local
installation and personal application data were left intact. User projects and
application data are kept by uninstall.

### Remaining manual checks

- Install or upgrade Windows with the delivered 1.0.1 installer, inspect its
  branded pages and unknown-publisher prompt, and check installed-app launch.
- On macOS and Linux, test actual native pickers, interactive installation,
  security prompts, uninstall, and installed-app window-state restoration.
  Embedded-driver tests queue dialog outcomes and do not operate the OS picker UI.
- Exercise actual Save/Discard outcomes, read-only destinations, and force-quit
  recovery on each target OS. Windows actual-dialog cancellation is verified;
  controller failure/discard paths have unit coverage.
- Review the custom macOS DMG background and installed application icon on a Mac.
  The application uses a free ad hoc signature without publisher verification or
  notarization. Hands-on Gatekeeper behavior remains unverified.
- Track development-only npm advisories and the Linux GTK dependency warnings
  documented in [SECURITY.md](../SECURITY.md). The regular application excludes
  WebdriverIO, its embedded driver and debug test APIs.

Releases are published manually after validation. Mobile, automatic updates,
paid publisher signing/notarization, OS associations, recent files, multiple
windows and automatic browser-profile migration remain deferred. Browser and
native storage are separate. Standard hosted runners are free for the public
repository; no paid certificate or signing account is used.
