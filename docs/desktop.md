# Brickpress desktop

Brickpress uses Tauri 2 with the existing statically prerendered SvelteKit editor.
No Node server is included. Windows uses WebView2, macOS uses WKWebView, and Linux
uses WebKitGTK. Native modules are loaded after client mount; browser builds keep
their existing inputs, downloads, localStorage, and IndexedDB.

## Prerequisites

- Node 20.19+ (CI uses Node 22), npm, stable Rust, rustfmt, and Clippy.
- Windows: `x86_64-pc-windows-msvc`, Visual Studio C++ Build Tools, Windows SDK,
  and Microsoft Edge WebView2 Runtime. NSIS packaging does not require WiX.
- macOS: Xcode command-line tools (`xcode-select --install`). Apple Silicon and
  Intel are built separately; the minimum supported macOS version is 11.
- Debian/Ubuntu: `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev
libssl-dev librsvg2-dev libayatana-appindicator3-dev patchelf`. Native headless
  tests also need `xvfb`.
- License generator: `cargo install cargo-about --version 0.9.2 --locked --features cli`.

Install JavaScript dependencies with `npm ci`. Rust dependencies are pinned in
`src-tauri/Cargo.lock`; build/test commands use `--locked`.

## Development and builds

```sh
npm run dev                    # browser edition
npm run desktop:dev            # native shell + Vite at 127.0.0.1:1420
npm run desktop:build          # native release + platform installer
```

Desktop development uses `strictPort`; close another service using port 1420
instead of allowing Tauri to connect to a different port. Native builds generate
JavaScript and Rust notices and then use the existing static `build/` output.
The notices are bundled as installer resources and readable from **Main menu →
Third-party notices**. Ordinary `npm run build` continues to build the browser.

Outputs are under `src-tauri/target/release/bundle/`. Select individual formats:

For the zero-budget distribution plan, see the [signing guide](signing.md).

```sh
npm run desktop:build -- --bundles nsis
npm run desktop:build -- --target aarch64-apple-darwin --bundles dmg
npm run desktop:build -- --target x86_64-apple-darwin --bundles dmg
npm run desktop:build -- --bundles appimage,deb
```

Cross-compiling does not replace the target OS's packaging tools. Build each
installer on its OS using the workflow below. Windows builds are unsigned;
macOS builds use a free ad hoc signature and are not notarized.

## Test installers and installation

Download the [v1.0.1 macOS/Linux test installers](https://github.com/Aqualy/Brickpress/releases/tag/v1.0.1)
or the preceding [v1.0.0 Windows test installer](https://github.com/Aqualy/Brickpress/releases/tag/v1.0.0).
The release pages include checksums and platform-specific instructions. See
[validation](desktop-validation.md) for automated results and remaining manual checks.

Run **Actions → Test installers → Run workflow**. Choose **macos-linux** for
Apple Silicon/Intel DMG, AppImage and Debian packages, or **all** to include
Windows NSIS. The workflow first runs browser and native checks, then uploads
installers as Actions artifacts for 14 days. It never creates a release or changes
repository visibility; validated installers can be uploaded to a test prerelease.
Download and extract the artifact for your OS. Each artifact contains
`SHA256SUMS.txt`; see the signing guide for verification commands. Standard hosted
runners are free for this public repository. If the repository becomes private,
keep paid Actions usage disabled before running workflows.

- Windows: run the NSIS setup executable. It installs for the current user;
  WebView2 is detected by the installer. Unsigned builds can show SmartScreen:
  after verifying the artifact's origin, use **More info → Run anyway**.
- macOS: mount the appropriate DMG and drag Brickpress to Applications. Ad hoc
  signed, unnotarized test builds can be blocked by Gatekeeper. For a trusted
  artifact use **System Settings → Privacy & Security → Open Anyway**.
- Linux: make the AppImage executable and run it, or install the Debian package
  with `sudo apt install ./Brickpress_*.deb`. AppImages may require the
  distribution's FUSE compatibility package.

Updates are manual: download and install the next test build. There is no
updater service, verified publisher signing, notarization, or automatic release publication.

## Application and installer artwork

`assets/branding/brickpress.png` preserves the supplied transparent artwork.
The generated icon set is used for the Windows executable and shortcuts, macOS
application, and Linux desktop packages. The same artwork appears in the browser
favicon, Windows setup/uninstaller icons and setup artwork, and the macOS DMG
background. Windows installer artwork uses native 24-bit BMP assets at the
recommended header and sidebar dimensions.

Regenerate these committed assets on Windows, after `npm ci`, with:

```powershell
pwsh -NoProfile -File scripts/generate-branding.ps1
```

The script uses the pinned Tauri CLI for icon conversion and Windows drawing
APIs for installer layouts. Other platforms consume the generated files without
needing PowerShell or another image-processing dependency.

The Windows packaging job verifies all icon resources in the application and
NSIS installer against the custom ICO, reading the executables directly without
Explorer's cached preview. To run that check locally after building:

```powershell
pwsh -NoProfile -File tests/native/installer-branding.ps1
```

## Documents and storage

Desktop Open/Save/Save As use native file dialogs. Save overwrites the active
selected file using atomic replacement and detects outside changes by content
fingerprint. A conflict offers Save As or Cancel. Save records the exact snapshot
written, so edits made during a save stay unsaved. Cancelled or failed saves keep
the current document and window. Project export is a copy and does not change
the active file or clear the unsaved indicator.

New/Open/Close use Save / Discard / Cancel for changed desktop projects. Discard
restores recovery to the last successful saved snapshot, or removes recovery for
a document never saved. Recovery reopened after a restart is unsaved and has no
active file; Save asks for a location. Invalid recovery is moved aside for
inspection. Storage failures are shown and keep the editor available for retry.

Recovery, composition presets, custom print presets, preferences and tracing assets live in the OS
application data directory for `io.github.aqualy.brickpress` (usually
`%APPDATA%\io.github.aqualy.brickpress` on Windows,
`~/Library/Application Support/io.github.aqualy.brickpress` on macOS, and
`~/.local/share/io.github.aqualy.brickpress` on Linux). Records use serialized
atomic writes. Trace metadata is committed only after its binary image asset;
older assets are pruned afterward. Files chosen in dialogs are represented by
short-lived opaque handles; no generic filesystem or shell permission is granted.

Browser and desktop storage stay separate. To move work, use **Save / Export
project** in the browser and **Open** in desktop. Export the browser preset
library and import it from desktop's Presets tab. Both `.brickpress.json` and
legacy `.legopress.json` version-1 projects are accepted. Tracing guides and
Flat/Embossed appearance remain editor preferences and are excluded from project
and artwork exports. Upload tracing images separately after migrating.

## Verification

```sh
npm run check
npm test
npm run validate:catalog
npm run build
npm run test:e2e
npm run notices:rust
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run desktop:build:e2e
npm run desktop:test
```

Native smoke tests use WebdriverIO's embedded Tauri driver on all three operating
systems, with a debug-only `desktop-e2e` feature. These builds use isolated
`artifacts/native/data` and do not load personal browser/desktop storage. Picker
and confirmation queues test the real command/controller path without interacting
with OS picker UI. Native driver/test commands are absent from regular builds;
release compilation explicitly rejects the test feature. CI runs native tests
on Windows, both macOS architectures and Linux (under Xvfb).

The optional real Windows dialog check uses PowerShell 7 UI Automation and an
interactive desktop, separately from the default embedded-driver suite:

```powershell
$env:BRICKPRESS_REAL_DIALOG_TESTS = '1'
npm run desktop:test -- --mochaOpts.grep 'real Windows'
```

It cancels actual Open/Save dialogs and an unsaved-close confirmation in the
isolated test application. Other dialog outcomes and installer behavior still
need the separate checks below on each target OS.

Real OS dialogs and installed application behavior require separate checks:

- Open and Save As using an actual native picker, including Unicode filenames,
  Cancel, read-only destinations, and an externally modified project.
- Save / Discard / Cancel before New/Open/Close/application Quit, and recovery after force quit.
- Install, launch, relaunch to focus the existing window, resize/maximize/restart,
  then uninstall without removing user projects.
- Design/Print Preview and seeded SVG filters; PNG and SVG with transparency.
- Supported tracing formats on the target webview, especially AVIF and GIF.
- Keyboard placement, middle-click sampling, tab/popover/drawer focus, 320px,
  enlarged text, accessibility and a 2,000-piece document.

See [desktop bridge](desktop-bridge.md) for the IPC contract. Mobile, automatic
updates, signing/notarization, OS associations, recent files, multiple document
windows, and browser-profile migration are outside this delivery. The catalog
and application licensing boundaries in `NOTICE.md` still apply to distribution.
