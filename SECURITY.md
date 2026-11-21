# Security

Brickpress is a local desktop application. Installers for Brickpress 2 are not
published yet. The 1.x test installers on
[GitHub Releases](https://github.com/Aqualy/Brickpress/releases) are unsigned on
Windows and ad hoc signed without notarization on macOS. They do not identify a
verified publisher.

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

## Dependencies

Brickpress 2 is a native Rust application. It has no web runtime, no Node.js
dependencies and no generic shell or filesystem access. Files are read and
written only after the person picks them in a system dialog, or, for the MCP
server, inside the folder given with `--root`. The live agent endpoint is a
per-user local socket. It is off until it is allowed in Help ▸ Allow Agent
Connections.

Dependency alerts remain enabled. Run `cargo audit` against `Cargo.lock` before
a release, and review any finding for each platform. A clean vulnerability
count does not establish that every transitive dependency is safe.
