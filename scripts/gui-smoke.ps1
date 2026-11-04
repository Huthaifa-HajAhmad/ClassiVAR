# Drives the real ClassiVAR window through the paths that have broken before:
# light/dark switching and its saved preference, gene fill-in suggestions, the
# PVS1 strength guide, and clearing the gene box (which must not wipe the case).
#
# It is deliberately paranoid about focus: it refuses to click or capture unless
# ClassiVAR is verifiably the front window, and captures only the client area.
#
# Controls are found by accessible name, so renaming these means updating the
# script: "Theme: light. Switch to dark.", "PVS1:*not applied*", "PVS1 Very
# strong:*", "PVS1 strength: Very strong*", "BRCA1". Clicking "Check gene" calls
# the live sources, so the test needs network access.
#
# Usage (Windows):
#   powershell -ExecutionPolicy Bypass -File scripts/gui-smoke.ps1
#   powershell -ExecutionPolicy Bypass -File scripts/gui-smoke.ps1 -Exe path\to\classivar.exe
# Screenshots are written to %TEMP%\classivar-smoke and the paths are printed.
param(
    [string]$Exe = "",
    [string]$OutDir = (Join-Path $env:TEMP "classivar-smoke")
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Exe)) {
    $root = Split-Path $PSScriptRoot -Parent
    $Exe = Join-Path $root "target\debug\classivar.exe"
}
if (-not (Test-Path $Exe)) { throw "ClassiVAR executable not found: $Exe (run: cargo build -p classivar)" }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName Microsoft.VisualBasic
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class ClassiVarSmokeWin {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
}
"@

$settings = Join-Path $env:APPDATA "ClassiVAR\settings.txt"
$hadSettings = Test-Path $settings
$originalSettings = if ($hadSettings) { Get-Content $settings -Raw } else { $null }
# Start from a known state: the smoke test checks light -> dark -> light.
New-Item -ItemType Directory -Force -Path (Split-Path $settings) | Out-Null
Set-Content -Path $settings -Value "theme=light" -NoNewline

$proc = Start-Process -FilePath $Exe -PassThru
try {
    for ($i = 0; $i -lt 150; $i++) {
        Start-Sleep -Milliseconds 200
        $proc.Refresh()
        if ($proc.MainWindowHandle -ne 0) { break }
    }
    $h = $proc.MainWindowHandle
    if ($h -eq 0) { throw "ClassiVAR did not open a window" }
    $shots = @()

    function Focus-App {
        for ($i = 0; $i -lt 20; $i++) {
            if ([ClassiVarSmokeWin]::GetForegroundWindow() -eq $h) { return }
            try { [Microsoft.VisualBasic.Interaction]::AppActivate($proc.Id) } catch {}
            [ClassiVarSmokeWin]::ShowWindow($h, 9) | Out-Null
            [ClassiVarSmokeWin]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
            [ClassiVarSmokeWin]::SetForegroundWindow($h) | Out-Null
            [ClassiVarSmokeWin]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
            Start-Sleep -Milliseconds 250
        }
        throw "ClassiVAR is not the front window; refusing to act"
    }
    function Root { [System.Windows.Automation.AutomationElement]::FromHandle($h) }
    function Button-Condition([string]$name) {
        New-Object System.Windows.Automation.AndCondition(
            (New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                [System.Windows.Automation.ControlType]::Button)),
            (New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::NameProperty, $name)))
    }
    function Find-Button([string]$name) {
        for ($i = 0; $i -lt 25; $i++) {
            $el = (Root).FindFirst([System.Windows.Automation.TreeScope]::Descendants, (Button-Condition $name))
            if ($null -ne $el) { return $el }
            Start-Sleep -Milliseconds 150
        }
        throw "button not found: $name"
    }
    function Find-Button-Like([string]$name) {
        for ($i = 0; $i -lt 25; $i++) {
            $buttons = (Root).FindAll(
                [System.Windows.Automation.TreeScope]::Descendants,
                (New-Object System.Windows.Automation.PropertyCondition(
                    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                    [System.Windows.Automation.ControlType]::Button)))
            $hit = @($buttons | Where-Object { $_.Current.Name -like $name })
            if ($hit.Count -gt 0) { return $hit[0] }
            Start-Sleep -Milliseconds 150
        }
        $seen = @($buttons | ForEach-Object { $_.Current.Name } | Where-Object { $_ -like "*PVS1*" }) -join " | "
        throw "button not found like: $name. PVS1 names seen: $seen"
    }
    function Invoke-Element($el) {
        try {
            $el.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
        } catch {
            # Some custom-drawn buttons do not expose Invoke. Fall back to a real
            # mouse click, but only after checking focus again: a click at a stale
            # position would land in another window.
            Focus-App
            $r = $el.Current.BoundingRectangle
            [ClassiVarSmokeWin]::SetCursorPos([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2)) | Out-Null
            [ClassiVarSmokeWin]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
            [ClassiVarSmokeWin]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
        }
        Start-Sleep -Milliseconds 450
    }
    function Click-Button([string]$name) {
        Focus-App
        Invoke-Element (Find-Button $name)
    }
    function Click-Button-Like([string]$name) {
        Focus-App
        Invoke-Element (Find-Button-Like $name)
    }
    # A text box is found by its placeholder, which is also its accessible name.
    function Find-Box([string]$placeholder) {
        $cond = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::NameProperty, $placeholder)
        return (Root).FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
    }
    function Gene-Box { return Find-Box "Gene symbol, e.g. BRCA1" }
    function Set-GeneText([string]$text) { Set-BoxText "Gene symbol, e.g. BRCA1" $text }
    function Press-Key([string]$keys) {
        Focus-App
        [System.Windows.Forms.SendKeys]::SendWait($keys)
        Start-Sleep -Milliseconds 450
    }
    function Press-Escape { Press-Key "{ESC}" }
    function Set-BoxText([string]$placeholder, [string]$text) {
        Focus-App
        $box = Find-Box $placeholder
        if ($null -eq $box) { throw "text box not found: $placeholder" }
        $box.SetFocus()
        Start-Sleep -Milliseconds 200
        Focus-App
        [System.Windows.Forms.SendKeys]::SendWait("^a")
        Start-Sleep -Milliseconds 120
        [System.Windows.Forms.SendKeys]::SendWait("{BACKSPACE}")
        Start-Sleep -Milliseconds 120
        if ($text -ne "") { [System.Windows.Forms.SendKeys]::SendWait($text) }
        Start-Sleep -Milliseconds 650
    }
    function Capture([string]$name) {
        Focus-App
        $r = New-Object ClassiVarSmokeWin+RECT
        [ClassiVarSmokeWin]::GetClientRect($h, [ref]$r) | Out-Null
        $p = New-Object ClassiVarSmokeWin+POINT
        $p.X = 0; $p.Y = 0
        [ClassiVarSmokeWin]::ClientToScreen($h, [ref]$p) | Out-Null
        $bmp = New-Object System.Drawing.Bitmap($r.Right, $r.Bottom)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen($p.X, $p.Y, 0, 0, (New-Object System.Drawing.Size($r.Right, $r.Bottom)))
        $path = Join-Path $OutDir "$name.png"
        $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
        $c = $bmp.GetPixel(20, $r.Bottom - 80)
        $g.Dispose(); $bmp.Dispose()
        $script:shots += $path
        return [pscustomobject]@{ Path = $path; Background = $c }
    }
    function Names {
        return @((Root).FindAll([System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition) | ForEach-Object { $_.Current.Name })
    }

    Focus-App
    # The first frame can be captured before the window has painted, so look again for a few seconds.
    $light = Capture "01-light"
    for ($i = 0; $i -lt 15 -and $light.Background.R -lt 200; $i++) {
        Start-Sleep -Milliseconds 400
        $light = Capture "01-light"
    }
    if ($light.Background.R -lt 200) { throw "the app did not start in the light theme" }

    Click-Button-Like "Theme:*Switch to dark*"
    $dark = Capture "02-dark"
    if ($dark.Background.R -gt 80) { throw "the background did not change from light to dark" }
    $saved = (Get-Content $settings -Raw).Trim()
    if ($saved -ne "theme=dark") { throw "the theme choice was not saved (settings: $saved)" }
    Find-Button-Like "Theme: dark. Switch to light.*" | Out-Null

    Set-GeneText "brc"
    $suggest = Capture "03-suggestions"
    Find-Button "BRCA1" | Out-Null
    Find-Button "BRCA2" | Out-Null
    Click-Button "BRCA1"
    $value = (Gene-Box).GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value
    if ($value -ne "BRCA1") { throw "the suggestion did not fill the gene box (got '$value')" }

    # PVS1 now asks which situation fits instead of applying Very strong.
    Click-Button-Like "PVS1:*not applied*"
    if (@(Names | Where-Object { $_ -like "PVS1 Very strong:*" }).Count -eq 0) {
        # A double click toggles the guide closed; retry once so a stray repeat
        # from the automation does not fail the run.
        Click-Button-Like "PVS1:*not applied*"
    }
    $guide = Capture "04-pvs1-guide"
    Find-Button-Like "PVS1 does not apply:*" | Out-Null
    Find-Button-Like "PVS1 Very strong:*" | Out-Null
    # Escape closes the guide without answering it.
    Press-Escape
    if (@(Names | Where-Object { $_ -like "PVS1 Very strong:*" }).Count -ne 0) {
        throw "Escape did not close the PVS1 guide"
    }
    if (@(Names | Where-Object { $_ -like "PVS1:*applied*" -and $_ -notlike "*not applied*" }).Count -ne 0) {
        throw "Escape applied PVS1 instead of just closing the guide"
    }
    Click-Button-Like "PVS1:*not applied*"
    Find-Button-Like "PVS1 Very strong:*" | Out-Null
    # The first option is visible without scrolling; later ones may be below the fold.
    Click-Button-Like "PVS1 Very strong:*"
    Find-Button-Like "PVS1:*applied*" | Out-Null
    Find-Button-Like "PVS1 strength: Very strong*" | Out-Null
    $after = Capture "05-pvs1-supporting"

    # The predictor helper turns a REVEL score into PP3 at the published strength.
    Click-Button "Open the predictor score helper"
    Set-BoxText "Score, e.g. 0.85" "0.95"
    Click-Button-Like "Apply PP3 Strong*"
    Find-Button-Like "PP3:*applied*" | Out-Null
    Find-Button-Like "PP3 strength: Strong*" | Out-Null
    # A score between the limits gives no button, and a benign score above
    # this program's limit is lowered to Moderate.
    Set-BoxText "Score, e.g. 0.85" "0.5"
    if (@(Names | Where-Object { $_ -like "Apply PP3*" -or $_ -like "Apply BP4*" }).Count -ne 0) {
        throw "a score between the limits still offered to apply a criterion"
    }
    Set-BoxText "Score, e.g. 0.85" "0.01"
    Find-Button-Like "Apply BP4 Moderate*" | Out-Null
    $predictor = Capture "05b-predictor"
    Click-Button "Close the predictor score helper"

    # The functional helper: controls route, then the OddsPath route.
    Click-Button "Open the functional evidence helper"
    Set-BoxText "Controls, e.g. 11" "12"
    Click-Button-Like "Apply PS3 Moderate*"
    Find-Button-Like "PS3:*applied*" | Out-Null
    Click-Button "OddsPath value"
    Set-BoxText "OddsPath, e.g. 5.2" "400"
    Find-Button-Like "Apply PS3 Very strong*" | Out-Null
    Set-BoxText "OddsPath, e.g. 5.2" "1"
    if (@(Names | Where-Object { $_ -like "Apply PS3*" -or $_ -like "Apply BS3*" }).Count -ne 0) {
        throw "an OddsPath in the indeterminate range still offered to apply a criterion"
    }
    $functional = Capture "05c-functional"
    Click-Button "Close the functional evidence helper"

    # Raising a strength above its default asks for a reason first.
    Click-Button-Like "PM1:*not applied*"
    Click-Button-Like "PM1 strength: Moderate*"
    $raise = Capture "05d-raise-reason"
    Set-BoxText "Why is this stronger than the default?" "Hot spot in a well-studied domain"
    Click-Button-Like "Save the reason and raise PM1*"
    Find-Button-Like "PM1 strength: Strong*" | Out-Null
    # Strong -> Supporting -> Moderate (the default) need no reason; the next
    # click raises again and asks. Escape then leaves the strength unchanged.
    Click-Button-Like "PM1 strength:*"
    Click-Button-Like "PM1 strength:*"
    Find-Button-Like "PM1 strength: Moderate*" | Out-Null
    Click-Button-Like "PM1 strength:*"
    Find-Button-Like "Save the reason and raise PM1*" | Out-Null
    Press-Escape
    Find-Button-Like "PM1 strength: Moderate*" | Out-Null
    if (@(Names | Where-Object { $_ -like "Save the reason and raise*" }).Count -ne 0) {
        throw "Escape did not close the reason prompt"
    }
    # Clicking the criterion code removes it, and its open reason prompt goes with it.
    Click-Button-Like "PM1 strength:*"
    Find-Button-Like "Save the reason and raise PM1*" | Out-Null
    Click-Button-Like "PM1:*applied*"
    Find-Button-Like "PM1:*not applied*" | Out-Null
    if (@(Names | Where-Object { $_ -like "Save the reason and raise*" }).Count -ne 0) {
        throw "removing PM1 left its reason prompt open"
    }

    # Page Down and Page Up scroll the list under the mouse pointer.
    $ps1 = Find-Button-Like "PS1:*"
    $r = $ps1.Current.BoundingRectangle
    $before = $r.Y
    [ClassiVarSmokeWin]::SetCursorPos([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2)) | Out-Null
    Press-Key "{PGDN}"
    $down = (Find-Button-Like "PS1:*").Current.BoundingRectangle.Y
    if ($down -ge $before) { throw "Page Down did not scroll the criteria list" }
    Press-Key "{PGUP}"
    $up = (Find-Button-Like "PS1:*").Current.BoundingRectangle.Y
    if ([math]::Abs($up - $before) -gt 2) { throw "Page Up did not scroll the criteria list back" }

    # Clearing the gene box and pressing Enter must not offer to wipe the case.
    Set-GeneText ""
    [System.Windows.Forms.SendKeys]::SendWait("{ENTER}")
    Start-Sleep -Milliseconds 500
    $empty = Capture "06-empty-gene"
    if (@(Names | Where-Object { $_ -eq "Yes, continue" }).Count -ne 0) {
        throw "an empty gene box still offers to clear the case"
    }
    if (@(Names | Where-Object { $_ -like "PVS1:*applied*" }).Count -ne 1) {
        throw "the case evidence was lost when the gene box was cleared"
    }

    # The path that once overflowed the stack: Reported variant, then the example.
    Click-Button "Reported variant"
    Click-Button "Load example"
    Start-Sleep -Milliseconds 400
    if (@(Names | Where-Object { $_ -eq "Yes, continue" }).Count -eq 1) {
        Click-Button "Yes, continue"
    }
    Find-Button-Like "Lab A*" | Out-Null
    # Removing a record must ask first, not delete on one click.
    Click-Button-Like "Lab A*"
    # The name box renames the record as you type.
    Set-BoxText "Lab or source, e.g. Lab A (ClinVar)" "Quest Dx"
    Find-Button-Like "Quest Dx*" | Out-Null
    # Put the criteria list at the top, so the Page Down check below has room to move it.
    $c = (Find-Button-Like "PS1:*").Current.BoundingRectangle
    [ClassiVarSmokeWin]::SetCursorPos([int]($c.X + $c.Width / 2), [int]($c.Y + $c.Height / 2)) | Out-Null
    Press-Key "{PGUP}"
    Press-Key "{PGUP}"
    Press-Key "{PGUP}"
    $ps1Top = (Find-Button-Like "PS1:*").Current.BoundingRectangle.Y
    Click-Button "Remove record"
    Start-Sleep -Milliseconds 300
    if (@(Names | Where-Object { $_ -eq "Yes, continue" }).Count -ne 1) {
        throw "Remove record did not ask for confirmation"
    }
    $modal = Capture "07a-confirm-window"
    # Page Down must not scroll the list behind the confirmation window.
    $c = (Find-Button-Like "PS1:*").Current.BoundingRectangle
    [ClassiVarSmokeWin]::SetCursorPos([int]($c.X + $c.Width / 2), [int]($c.Y + $c.Height / 2)) | Out-Null
    Press-Key "{PGDN}"
    $ps1Behind = (Find-Button-Like "PS1:*").Current.BoundingRectangle.Y
    if ($ps1Behind -ne $ps1Top) { throw "Page Down scrolled the list behind the confirmation window" }
    # Escape answers Cancel, and the record is still there.
    Press-Escape
    if (@(Names | Where-Object { $_ -eq "Yes, continue" }).Count -ne 0) {
        throw "Escape did not close the confirmation"
    }
    Find-Button-Like "Quest Dx*" | Out-Null
    Click-Button "Remove record"
    Click-Button "Cancel"
    $reported = Capture "07-reported"

    Click-Button-Like "Theme:*Switch to light*"
    $back = Capture "08-light-again"
    if ($back.Background.R -lt 200) { throw "the background did not return to light" }

    Write-Host "GUI smoke passed. Screenshots:" -ForegroundColor Green
    $shots | ForEach-Object { Write-Host "  $_" }
}
finally {
    $proc.CloseMainWindow() | Out-Null
    Start-Sleep -Milliseconds 500
    if (-not $proc.HasExited) { $proc.Kill() }
    # Leave the curator's theme exactly as it was before the smoke test.
    if ($hadSettings) { Set-Content -Path $settings -Value $originalSettings -NoNewline }
    elseif (Test-Path $settings) { Remove-Item $settings -Force }
}
