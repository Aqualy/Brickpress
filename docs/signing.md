# Signing Brickpress

Provider details checked on 7 October 2026. Brickpress follows a zero-budget
distribution plan: unsigned Windows installers, ad hoc signed macOS applications,
and Linux packages. No publisher account or certificate has been purchased.

## Current plan: no paid signing

- **Windows:** distribute the NSIS installer without Authenticode signing.
  Windows can show an unknown publisher and SmartScreen warnings. Some managed
  devices block these downloads. A self-signed certificate would still require
  users to change certificate trust; it does not solve public publisher trust.
- **macOS:** `src-tauri/tauri.macos.conf.json` sets `signingIdentity` to `-`.
  Tauri applies a free ad hoc signature, including on Apple Silicon. This does
  not verify a publisher identity or notarize the application. Users may need
  **System Settings → Privacy & Security → Open Anyway** after trying to launch
  a trusted download. See [Tauri's ad hoc signing instructions](https://v2.tauri.app/distribute/sign/macos/#ad-hoc-signing).
- **Linux:** distribute AppImage and Debian packages directly. Paid certificates
  are not required for these downloads.

The test installer workflow places `SHA256SUMS.txt` beside each platform's
packages. On macOS/Linux, run `shasum -a 256 -c SHA256SUMS.txt` from the extracted
artifact directory. On Windows, use `Get-FileHash -Algorithm SHA256` for the
installer and compare the result with the manifest. These checks detect changed
bytes; an unsigned checksum does not establish a verified publisher identity.

The repository is public, where standard hosted runners are free. If it becomes
private, keep GitHub Actions paid usage disabled (a $0 spending limit): private
repositories consume the account's included allowance. Stop or postpone builds
if that allowance is exhausted. Releases are published manually after validation.
See [GitHub's runner pricing rules](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

There is a possible free Windows Store route: add **MSIX** packaging, enroll in
the Store's free developer program, and pass certification. Microsoft signs
certified MSIX packages. Brickpress currently produces NSIS EXE installers, so
this needs separate implementation and submission. The Store's EXE/MSI route
still requires publisher signing. See
[Microsoft's comparison](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options).

The paid setup below is reference material for a later budget change; it is not
required for the current plan.

Signing attaches a verified publisher identity and a tamper-evident signature
to a build. A timestamp preserves signature validity after the signing
certificate expires. macOS also uses Apple's notarization service. The signing
credentials for Windows and macOS are separate.

## Windows: an individual in Belgium

Microsoft's Azure Artifact Signing Public Trust service currently supports
individuals only in the US and Canada. EU organizations are eligible, but an
individual developer in Belgium cannot use that route. See Microsoft's
[eligibility requirements](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart).

For direct EXE downloads, use a certificate provider that verifies individual
developers and supports cloud signing for GitHub Actions. SSL.com's
[Individual Validated code signing](https://www.ssl.com/products/software-integrity/code-signing/iv/)
is one option. Its page lists US$129 for a one-year certificate, plus a separate
cloud-signing subscription or hardware-key cost. Verify the complete quote and
Belgian individual eligibility with the provider before ordering.

1. Create the provider account using your legal name. That verified name will
   appear as the Windows publisher; the app can still be called Brickpress.
2. Choose individual code signing and cloud signing that supports automated
   builds. Obtain the certificate and signing-service prices together.
3. Complete the provider's identity check, including the requested ID documents.
4. Activate the certificate in the cloud-signing service and obtain its CI
   credentials and certificate identifier through the provider's dashboard.
5. Store credentials in GitHub repository or protected release-environment
   secrets. Keep private keys, passwords and identity documents out of commits
   and chat messages.
6. Configure Tauri's Windows `signCommand` with the provider's signing client.
   The release pipeline must sign the app and installer, timestamp signatures,
   and verify them before uploading the final files.

New signed software can still show SmartScreen warnings while reputation builds.
EV certificates do not provide an immediate bypass. See
[Microsoft's signing comparison](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)
and [Tauri's Windows signing guide](https://v2.tauri.app/distribute/sign/windows/).

## Windows: a registered organization

If publishing through a registered Belgian company or another eligible EU
organization, consider Azure Artifact Signing. Microsoft's comparison lists
approximately US$9.99/month for the Basic plan; verify current regional pricing.

1. Create an Azure subscription and Microsoft Entra tenant.
2. Create an Artifact Signing account, complete organization identity
   validation, and create a Public Trust certificate profile.
3. Give the GitHub release workflow access to that profile, preferably through
   GitHub-to-Azure OIDC authentication with permission limited to signing.
4. Configure the account endpoint, certificate profile and Tauri `signCommand`.
5. Build, timestamp, verify and distribute the signed app and installer.

Follow Microsoft's
[Artifact Signing setup](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart).

## macOS

1. Enroll in the [Apple Developer Program](https://developer.apple.com/programs/enroll/)
   as an individual or organization. Individual enrollment uses your legal name,
   an Apple Account with two-factor authentication, and identity/contact details.
   Apple lists US$99 per membership year; local currency and regional prices are
   shown during enrollment.
2. Create a certificate signing request. Apple's
   [Keychain Access instructions](https://developer.apple.com/help/account/certificates/create-a-certificate-signing-request)
   describe the process on a Mac. If working only from Windows, the certificate
   request/key setup can be prepared separately; macOS GitHub runners will build
   and sign the application.
3. In Apple Developer **Certificates, Identifiers & Profiles**, create a
   **Developer ID Application** certificate and upload the request. Follow
   [Apple's certificate instructions](https://developer.apple.com/help/account/certificates/create-developer-id-certificates/).
4. Export the certificate together with its private key as a password-protected
   `.p12`. Keep a private backup. The certificate without its private key cannot
   sign builds.
5. Set up notarization credentials. Tauri supports an App Store Connect API key
   or an Apple ID with an app-specific password and Team ID. An API key includes
   its issuer ID, key ID and `.p8` private key.
6. Add the signing and notarization credentials to protected GitHub secrets.
   Tauri uses `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
   `APPLE_SIGNING_IDENTITY`, and either the Apple ID variables or the
   `APPLE_API_ISSUER`, `APPLE_API_KEY` and `APPLE_API_KEY_PATH` variables. The
   workflow writes the API private key to a temporary file and passes its path.
7. Import the certificate into a temporary CI keychain, build the Intel and
   Apple Silicon applications, sign them, submit for notarization, staple the
   resulting ticket, and verify the packages before publishing the DMGs.

See [Tauri's macOS signing and notarization guide](https://v2.tauri.app/distribute/sign/macos/).
Apple account enrollment and identity checks must be completed by the publisher;
the build and signing pipeline can then perform the repeated release steps.

## Linux

AppImage and Debian packages can be distributed directly. For release integrity,
publish SHA-256 checksums and sign the checksum manifest using a maintained
project signing key. Repository/package-store distribution can add its own
signing requirements. See [Tauri's Linux signing guide](https://v2.tauri.app/distribute/sign/linux/).

## ACSL and free signing programs

Brickpress's original code and catalog use ACSL 1.4. The
[ACSL authors](https://anticapitalist.software/) state that it is not an
open-source license. [SignPath Foundation](https://signpath.org/terms.html)
requires an OSI-approved open-source license for its free program, so that
program does not fit this license choice.

The Microsoft Store offers a separate Windows route: it signs certified MSIX
submissions. That would require adding MSIX packaging and a Store listing;
Brickpress currently builds an NSIS EXE installer. The Store's MSI/EXE route
still requires publisher signing. See Microsoft's signing comparison above.
