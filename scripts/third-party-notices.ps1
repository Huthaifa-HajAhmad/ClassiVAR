# Writes THIRD-PARTY-NOTICES.md (every package ClassiVAR builds with, its version and
# licence) and THIRD-PARTY-LICENSES.html (the licence texts, gathered by cargo-about;
# install it once with `cargo install cargo-about --locked --features cli`). Both ship
# inside the installers. Run from the repository root after changing Cargo.lock:
#   powershell -ExecutionPolicy Bypass -File scripts/third-party-notices.ps1
$ErrorActionPreference = "Stop"
$ours = @("classivar", "classivar-core", "classivar-sources")
# The packages that end up in the program: normal dependencies on every platform. Build
# scripts and test-only packages are left out, the same as in the licence texts
# (about.toml). `{p}` is "name vX.Y.Z (details)" and `{l}` is the licence expression.
$tree = cargo tree --workspace --locked -e normal --target all --prefix none --format "{p}|{l}"
if ($LASTEXITCODE -ne 0) { throw "cargo tree failed" }
# cargo tree puts a blank line between the workspace packages; those lines have no "|".
$packages = $tree | Where-Object { $_ -match "\|" } | ForEach-Object {
    $parts = $_ -split "\|", 2
    $id = $parts[0] -split " "
    [pscustomobject]@{ name = $id[0]; version = $id[1].TrimStart("v"); license = $parts[1].Trim() }
} | Where-Object { $_.name -notin $ours } |
    Group-Object name, version | ForEach-Object { $_.Group[0] } | Sort-Object name, version
$lines = @(
    "# Third-party notices",
    "",
    "ClassiVAR itself is licensed under MIT OR Apache-2.0 (see LICENSE-MIT and LICENSE-APACHE).",
    "It ships with the packages below (build scripts and test-only packages are not listed). This",
    "is a list of names, versions and licences taken from Cargo. The licence texts are in",
    "THIRD-PARTY-LICENSES.html, which ships with the installers. Where a package offers a choice that includes GPL or LGPL, ClassiVAR uses",
    "its MIT or Apache-2.0 option.",
    "",
    "Packages offered under several licences (``A OR B``) can be used under any one of them.",
    "Packages under MPL-2.0 keep their own licence on their own files:",
    ""
)
$mpl = $packages | Where-Object { $_.license -match "MPL-2.0" }
foreach ($p in $mpl) { $lines += "- $($p.name) $($p.version)" }
$lines += @("", "| Package | Version | Licence |", "|---|---|---|")
foreach ($p in $packages) {
    $licence = if ($p.license) { $p.license } else { "not stated in Cargo metadata" }
    $lines += "| $($p.name) | $($p.version) | $licence |"
}
# UTF-8 without a byte-order mark, and LF line ends, so the file is the same on every system.
$text = ($lines -join "`n") + "`n"
[IO.File]::WriteAllText((Join-Path $PWD.Path "THIRD-PARTY-NOTICES.md"), $text, (New-Object System.Text.UTF8Encoding $false))
Write-Host "Wrote THIRD-PARTY-NOTICES.md ($($packages.Count) packages)."
cargo about generate about.hbs --locked -o THIRD-PARTY-LICENSES.html
if ($LASTEXITCODE -ne 0) { throw "cargo about could not gather the licence texts (see about.toml)" }
Write-Host "Wrote THIRD-PARTY-LICENSES.html."
