# Public repository audit

Reviewed on 8 October 2026 while preparing Brickpress 1.0.1 for friend testing.
The repository is public. The audit covered current tracked source, all branches
and tags in Git history, repository settings, bundled assets and dependencies.

## Source and history

Gitleaks 8.30.1 scanned all 17 existing commits before the public-repository
cleanup and the complete current source tree, including the new safeguards.
Both scans reported zero findings. The subsequent
[Repository audit workflow](https://github.com/Aqualy/Brickpress/actions/runs/37825826900)
passed on the cleanup commit with full-history scanning enabled.

Tracked files and historical filenames contained no credentials, signing keys,
personal project exports, recovery records, local editor settings or generated
installers. Build output, test artifacts and downloaded audit tools remain local
and ignored. All commit identities used a GitHub noreply address; no personal
email address was found in commit authors or committers.

The 20 tracked PNG assets had no EXIF, XMP or PNG text metadata. Branding artwork,
catalog illustrations and small tracing test fixtures are intentional source
assets. Supplied reference screenshots and personal tracing images are not
included. The vendored native test driver has its license notices and is excluded
from regular installers.

No secret removal or history rewrite was needed. The audit did not find material
requiring deletion from the published repository. Automated detection and this
review cannot prove the absence of every possible secret.

## Ongoing safeguards

- `.gitignore` excludes personal document/preset exports, recovery/preferences,
  credentials, signing materials, generated packages, logs and local tool state.
- `scripts/check-repository.mjs` checks tracked filenames and optionally all Git
  history. A separate test index confirmed that it rejects a signing-key file.
- The Repository audit workflow checks full history and uses a pinned Gitleaks
  binary whose download is verified with SHA-256.
- GitHub secret scanning, push protection, dependency alerts, Dependabot security
  updates and private vulnerability reporting are enabled. The secret-alert
  API returned no alerts during this review.

## Dependencies and delivery

The production npm audit reports zero known vulnerabilities. Development tooling
has 16 affected dependency entries rooted in two packages without patched
upstream releases. The Cargo audit reports zero vulnerability-category advisories
and two informational warnings in Linux's GTK dependency chain. These are open,
documented findings; see [SECURITY.md](../SECURITY.md) for versions, advisory links
and platform scope.

The Windows 1.0.1 production installer passed version, custom-icon and packaged
notice checks. Release installer sizes and SHA-256 digests are verified against
GitHub's uploaded asset metadata. The release remains a prerelease for friend
testing. Windows is unsigned; macOS uses an ad hoc signature without notarization.
Interactive installation and upgrade checks remain as described in
[desktop validation](desktop-validation.md).

The ACSL license and the original catalog were retained. Third-party notices
accompany the installers; this audit does not replace review of their license
terms before changing distribution or dependencies.
