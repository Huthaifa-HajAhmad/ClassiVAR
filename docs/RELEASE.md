# Releasing ClassiVAR

How to build the installers and publish a release. The release notes are in
`CHANGELOG.md`.

## What a release is

- A Windows installer (`classivar_<version>_x64-setup.exe`, NSIS, installs for
  the current user without administrator rights) and a macOS disk image
  (`.dmg` holding `ClassiVAR.app`).
- Both carry `LICENSE-MIT`, `LICENSE-APACHE`, `THIRD-PARTY-NOTICES.md` and
  `THIRD-PARTY-LICENSES.html`.
- The rules stay `UNVERIFIED` until every entry of `rules/review-signoff.toml`
  has a human sign-off. A release never changes that, and the app says so on
  every class.

## Checklist

1. `docs/ROADMAP.md` has no open item marked as blocking a release.
2. The version in `Cargo.toml` (`[workspace.package]`) and the top entry of
   `CHANGELOG.md` match.
3. After any change to `Cargo.lock`: run
   `powershell -ExecutionPolicy Bypass -File scripts/third-party-notices.ps1`
   and commit both generated files.
4. The merge gate passes: `scripts/check.ps1`.
5. The live sources answer: `cargo test -p classivar-sources -- --ignored`.
6. The Windows UI smoke test passes on a free machine: `scripts/gui-smoke.ps1`.
7. Build the Windows installer locally (below), install it, open it from the
   Start menu, check a real variant, then uninstall it from Windows settings.
8. Push a tag `v<version>`. The `release` workflow builds both installers and
   attaches them to a **draft** release.
9. Download both files from the draft, try them on a clean Windows and a
   clean Mac, then publish the release.

## Building locally

Once: `cargo install cargo-packager --locked` (and, for the licence texts,
`cargo install cargo-about --locked --features cli`).

```powershell
cargo packager -p classivar --release -f nsis       # Windows, on Windows
cargo packager -p classivar --release -f app,dmg    # macOS, on a Mac
```

The files land in `target/packages/`. The settings are under
`[package.metadata.packager]` in `crates/classivar/Cargo.toml`; the icons are in
`crates/classivar/assets/icons/`, and `crates/classivar/build.rs` puts the icon
into the Windows program file.

## Signing

Unsigned installers work, but Windows shows a SmartScreen warning and macOS
blocks the app until the user allows it. To sign, add these repository secrets.
The `release` workflow signs when they exist, warns when they do not, and, when
they do, **checks the signature on the real files** and fails if it is not valid
(it never infers "signed" from the secrets).

- Windows (code-signing certificate): `WINDOWS_CERTIFICATE` (the `.pfx` file,
  base64), `WINDOWS_CERTIFICATE_PASSWORD`. The installer is signed with
  `signtool`; the check reads `Get-AuthenticodeSignature` and wants `Valid`.
- macOS (Developer ID Application certificate): `APPLE_CERTIFICATE` (the `.p12`
  file, base64), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` (for
  example `Developer ID Application: Name (TEAMID)`).
- macOS notarization (without it the app is signed but macOS still warns):
  `APPLE_ID`, `APPLE_PASSWORD` (an app-specific password), `APPLE_TEAM_ID`.

How the macOS part works: cargo-packager signs and notarizes only when
`signing-identity` is in its config. The workflow appends that line to the
`[package.metadata.packager.macos]` table of `crates/classivar/Cargo.toml` (so that
table must stay the last one in the file; the workflow checks) and passes the
`APPLE_*` variables to the build step. cargo-packager imports the certificate into
its own keychain. Afterwards the workflow runs `codesign --verify --deep --strict`,
checks for a Developer ID authority, and, with notarization secrets, `spctl -a -vv`.
The build step without signing has no `APPLE_*` variables at all, because
cargo-packager treats an empty `APPLE_CERTIFICATE` as a certificate.

**None of the signing steps has run yet:** no certificate has been added. The
first signed release needs a hand check too (Windows: the file's Properties >
Digital Signatures; macOS: `spctl -a -vv ClassiVAR.app`).

## How the release is published

The `package` jobs (Windows, macOS) upload their installers as workflow
artifacts. One `publish` job then creates the draft release (once, so the two
platforms cannot race), uses this version's section of `CHANGELOG.md` as its
notes, refuses a tag that does not match the version in `Cargo.toml`, and attaches
both installers.
