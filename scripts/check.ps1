# Runs every check the project requires before a merge. Exits on the first failure.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/check.ps1
$ErrorActionPreference = 'Stop'
$env:CARGO_TERM_COLOR = 'always'

function Run([string[]]$command) {
    & $command[0] $command[1..($command.Length - 1)]
    if ($LASTEXITCODE -ne 0) {
        Write-Host "FAILED: $($command -join ' ')" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

Run @('cargo', 'fmt', '--all', '--check')
Run @('cargo', 'clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings')
Run @('cargo', 'test', '--workspace', '--locked')
Run @('cargo', 'build', '--workspace', '--locked')
Write-Host 'All checks passed.' -ForegroundColor Green
