# Writes THIRD-PARTY-NOTICES.md: every package ClassiVAR builds with, its version and licence.
# Run from the repository root after changing Cargo.lock:
#   powershell -ExecutionPolicy Bypass -File scripts/third-party-notices.ps1
$ErrorActionPreference = "Stop"
$json = cargo metadata --format-version 1 --locked | ConvertFrom-Json
$packages = $json.packages |
    Where-Object { $_.name -notin @("classivar", "classivar-core", "classivar-sources") } |
    Sort-Object name, version
$lines = @(
    "# Third-party notices",
    "",
    "ClassiVAR itself is licensed under MIT OR Apache-2.0 (see LICENSE-MIT and LICENSE-APACHE).",
    "It is built with the packages below. This is a list of names, versions and licences taken",
    "from Cargo metadata. It does **not** contain the licence texts: most of these licences",
    "require the text to ship with a binary, so gather the texts (for example with ``cargo about``)",
    "before any installer is published.",
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
Set-Content -Path "THIRD-PARTY-NOTICES.md" -Value $lines -Encoding utf8
Write-Host "Wrote THIRD-PARTY-NOTICES.md ($($packages.Count) packages)."
