<#
.SYNOPSIS
    Serves ClassiVAR HTML/CSS screen prototypes with live reload on localhost.
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts/prototype.ps1
    powershell -ExecutionPolicy Bypass -File scripts/prototype.ps1 -File my-idea.html -Port 4173 -NoOpen
#>
param(
    [int]$Port = 4173,
    [string]$File = "starter.html",
    [switch]$NoOpen
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$protoRoot = (Resolve-Path (Join-Path $repoRoot 'docs\design\prototype')).Path
$assetsRoot = (Resolve-Path (Join-Path $repoRoot 'crates\classivar\assets')).Path
$themeFile = Join-Path $assetsRoot 'classivar-theme.json'

$mimeTypes = @{
    '.html' = 'text/html; charset=utf-8'
    '.css'  = 'text/css; charset=utf-8'
    '.js'   = 'application/javascript; charset=utf-8'
    '.json' = 'application/json; charset=utf-8'
    '.ttf'  = 'font/ttf'
    '.svg'  = 'image/svg+xml'
    '.png'  = 'image/png'
    '.md'   = 'text/markdown; charset=utf-8'
}

function Get-PrototypeVersion {
    $maxTicks = [int64]0
    foreach ($item in (Get-ChildItem -Path $protoRoot -Recurse -File -ErrorAction SilentlyContinue)) {
        if ($item.LastWriteTimeUtc.Ticks -gt $maxTicks) { $maxTicks = $item.LastWriteTimeUtc.Ticks }
    }
    if (Test-Path $themeFile) {
        $themeTicks = (Get-Item $themeFile).LastWriteTimeUtc.Ticks
        if ($themeTicks -gt $maxTicks) { $maxTicks = $themeTicks }
    }
    return $maxTicks.ToString()
}

function Resolve-SafeFile([string]$Root, [string]$RelativePath) {
    $cleaned = ($RelativePath -replace '/', '\').TrimStart('\')
    $candidate = [System.IO.Path]::GetFullPath((Join-Path $Root $cleaned))
    $rootPrefix = $Root.TrimEnd('\') + '\'
    if (-not $candidate.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $null
    }
    return $candidate
}

function Send-Bytes([System.Net.HttpListenerResponse]$Response, [int]$StatusCode, [string]$ContentType, [byte[]]$Bytes) {
    $Response.StatusCode = $StatusCode
    $Response.ContentType = $ContentType
    $Response.Headers['Cache-Control'] = 'no-store'
    $Response.ContentLength64 = $Bytes.Length
    $Response.OutputStream.Write($Bytes, 0, $Bytes.Length)
    $Response.OutputStream.Close()
}

function Send-Response([System.Net.HttpListenerContext]$Context) {
    $path = [System.Uri]::UnescapeDataString($Context.Request.Url.AbsolutePath)

    if ($path -eq '/__version') {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes((Get-PrototypeVersion))
        Send-Bytes $Context.Response 200 'text/plain; charset=utf-8' $bytes
        return
    }

    if ($path -eq '/') { $path = "/$File" }

    $resolved = if ($path.StartsWith('/assets/', [System.StringComparison]::OrdinalIgnoreCase)) {
        Resolve-SafeFile $assetsRoot ($path.Substring('/assets/'.Length))
    } else {
        Resolve-SafeFile $protoRoot ($path.TrimStart('/'))
    }

    if ($null -eq $resolved) {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes('Forbidden')
        Send-Bytes $Context.Response 403 'text/plain; charset=utf-8' $bytes
        return
    }

    if (-not (Test-Path $resolved -PathType Leaf)) {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes('Not found')
        Send-Bytes $Context.Response 404 'text/plain; charset=utf-8' $bytes
        return
    }

    $ext = [System.IO.Path]::GetExtension($resolved).ToLowerInvariant()
    $type = if ($mimeTypes.ContainsKey($ext)) { $mimeTypes[$ext] } else { 'application/octet-stream' }
    $bytes = [System.IO.File]::ReadAllBytes($resolved)
    Send-Bytes $Context.Response 200 $type $bytes
}

$prefix = "http://127.0.0.1:$Port/"
$listener = [System.Net.HttpListener]::new()
$listener.Prefixes.Add($prefix)
$listener.Start()

try {
    $url = "$prefix$File"
    Write-Host "ClassiVAR prototype server running at $url (Ctrl+C to stop)" -ForegroundColor Cyan
    if (-not $NoOpen) {
        Start-Process $url
    }

    while ($listener.IsListening) {
        $task = $listener.GetContextAsync()
        while (-not $task.AsyncWaitHandle.WaitOne(200)) { }
        $context = $task.GetAwaiter().GetResult()
        # A browser that drops a connection while a page loads (routine with live reload)
        # makes the write fail. That must not stop the server.
        try {
            Send-Response $context
        }
        catch {
            Write-Host "Request failed, still serving: $($_.Exception.Message)" -ForegroundColor Yellow
            try { $context.Response.Abort() } catch { }
        }
    }
}
finally {
    $listener.Stop()
    $listener.Close()
}
