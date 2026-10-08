# Security

Brickpress is a local desktop/browser application. Current test installers are
available from [GitHub Releases](https://github.com/Aqualy/Brickpress/releases).
Use the latest test version and verify its SHA-256 checksum before installation.
Windows test builds are unsigned; macOS test builds use ad hoc signatures without
notarization. They do not identify a verified publisher.

## Report a vulnerability

Use [private vulnerability reporting](https://github.com/Aqualy/Brickpress/security/advisories/new)
for suspected security problems. Include the application version, operating
system, reproduction steps, and expected impact. Reports are handled on a
best-effort basis.

Ordinary bugs can go in [Issues](https://github.com/Aqualy/Brickpress/issues).
Do not put credentials, private projects, tracing images, or personal file paths
in public reports. Review logs and screenshots before attaching them.

## Repository safeguards

GitHub secret scanning, push protection, dependency security alerts/updates and
private reporting are enabled. The Repository audit workflow scans full Git
history using a pinned, checksum-verified Gitleaks binary and checks for generated
files, private keys, personal project exports and device recovery records.

To check tracked filenames locally:

```sh
node scripts/check-repository.mjs
node scripts/check-repository.mjs --history
```

These checks reduce accidental exposure; they do not prove that every possible
secret or vulnerability is absent. A leaked credential must be revoked, even if
its file is later deleted.

## Dependency audit, 8 October 2026

The production npm dependency audit reports zero known vulnerabilities. Regular
installers contain neither Node.js nor the WebdriverIO test driver. Native IPC
is limited to the main window and files explicitly selected through dialogs;
there is no generic shell or filesystem permission.

The development test chain still reports 16 affected dependency entries rooted
in two packages with no patched upstream release:

- `braces` 3.0.3, through Mocha's file watcher:
  [GHSA-vfj7-8cjw-p6xm](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm).
- `extract-zip` 2.0.1, through WebdriverIO/Puppeteer's browser downloader:
  [GHSA-jmr9-qjv8-65gv](https://github.com/advisories/GHSA-jmr9-qjv8-65gv) and
  [GHSA-7pqw-9j4j-h8q3](https://github.com/advisories/GHSA-7pqw-9j4j-h8q3).

The native suite uses its embedded driver, rather than browser downloads, and
does not run in watch mode. Keep its patterns and downloaded archives trusted.
Do not run the test tooling against arbitrary third-party archives or patterns.
Dependency alerts remain enabled so a compatible upstream fix can be reviewed.

The Cargo lockfile audit reports zero advisories in its vulnerability category,
and two informational warnings in the Linux GTK dependency chain:

- `proc-macro-error` 1.0.4 is unmaintained:
  [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html).
- `glib` 0.18.5 has an unsound `VariantStrIter` implementation that can cause
  crashes when those iterator methods are used:
  [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html).
  The upstream fix requires `glib` 0.20 or newer; Tauri's GTK3 dependency family
  currently uses 0.18. Brickpress does not directly use `VariantStrIter`, but
  this does not establish that no transitive dependency can reach it.

Neither package appears in the Windows production dependency graph. These
warnings remain open for Linux and are not suppressed in the audit. Review a
compatible upstream update before a general release; do not treat the zero
vulnerability count as a clean bill of health for every platform.
