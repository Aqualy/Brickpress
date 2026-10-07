# Embedded driver compatibility patch

The `tauri-plugin-wdio-webdriver` folder contains the MIT-licensed crates.io
source of WebdriverIO's embedded driver **1.4.0**, used only by the explicit
debug-only `desktop-e2e` Cargo feature. Keep its LICENSE with this source.

The upstream Windows manifest uses `windows`/`windows-core` 0.61 and
`webview2-com` 0.38, and omits `Win32_System_Com_StructuredStorage`. These fail to
compile with Tauri 2.12.1's WebView2 0.39 / Windows COM 0.62 interfaces.

The only changes are in the copied Cargo.toml: align those three versions with
Tauri and enable StructuredStorage. The driver source is unchanged. Cargo.lock
pins the remaining dependencies. Remove this patch once an upstream published
driver supports those interfaces. The production application does not compile
or initialize this dependency; release builds reject `desktop-e2e`.
