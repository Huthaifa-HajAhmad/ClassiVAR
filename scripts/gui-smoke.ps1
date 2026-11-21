# Drives the real ClassiVAR window through the paths that have broken before:
# light/dark switching and its saved preference, gene fill-in suggestions, the
# PVS1 strength guide, and clearing the gene box (which must not wipe the case).
#
# It is deliberately paranoid about focus: it refuses to click or capture unless
# ClassiVAR is verifiably the front window, and captures only the client area.
#
# Controls are found by accessible name, so renaming these means updating the
# script: "Theme: light. Switch to dark.", "PVS1:*not applied*", "PVS1 Very
# strong:*", "PVS1 strength: Very strong*", "BRCA1". Text boxes are found by
# their aria labels. The script never calls the gene sources, so it runs
# offline; the live source tests cover the network.
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
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
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
        # Show the buttons that start with the same word, so a wrong name is easy to spot.
        $first = $name.Split(' ')[0].TrimEnd(':').TrimEnd('*')
        $seen = @($buttons | ForEach-Object { $_.Current.Name } | Where-Object { $_ -like "$first*" }) -join " | "
        throw "button not found like: $name. Names that start with '$first': $seen"
    }
    # Where the criteria list sits on screen. The list starts below the pane header,
    # about 255 px under the top of the window; a row above that line is hidden.
    function Get-ListArea {
        $client = New-Object ClassiVarSmokeWin+RECT
        [ClassiVarSmokeWin]::GetClientRect($h, [ref]$client) | Out-Null
        $origin = New-Object ClassiVarSmokeWin+POINT
        [ClassiVarSmokeWin]::ClientToScreen($h, [ref]$origin) | Out-Null
        $top = $origin.Y + 255
        $bottom = $origin.Y + $client.Bottom
        $listWidth = [math]::Min(465, [math]::Max(340, $client.Right * 0.36))
        return [pscustomobject]@{
            Top = $top; Bottom = $bottom; Left = $origin.X + $client.Right - $listWidth
            PointerX = $origin.X + $client.Right - 150; PointerY = [int](($top + $bottom) / 2)
        }
    }
    # Page keys scroll the pane under the pointer, so put the pointer over the list first.
    function Point-At-List {
        $a = Get-ListArea
        [ClassiVarSmokeWin]::SetCursorPos($a.PointerX, $a.PointerY) | Out-Null
    }
    # The result pane scrolls with the same keys when the pointer is over it.
    function Point-At-Result([int]$y = 520) {
        $origin = New-Object ClassiVarSmokeWin+POINT
        [ClassiVarSmokeWin]::ClientToScreen($h, [ref]$origin) | Out-Null
        $client = New-Object ClassiVarSmokeWin+RECT
        [ClassiVarSmokeWin]::GetClientRect($h, [ref]$client) | Out-Null
        [ClassiVarSmokeWin]::SetCursorPos([int]($origin.X + 150), [int]($origin.Y + $y)) | Out-Null
    }
    # A click on a button whose centre is below the window does nothing. Like a
    # person would, scroll the pane the control belongs to (Page Up or Page Down
    # with the pointer over it) until the centre is on screen. This is only a safety
    # net for the generic click helpers: a row of the criteria list can also hide
    # under the pane header, and Scroll-To-Button (which knows where the list
    # starts) is used for those.
    function Bring-Into-View($el) {
        for ($i = 0; $i -lt 10; $i++) {
            $r = $el.Current.BoundingRectangle
            $area = Get-ListArea
            if (($r.Y + $r.Height / 2) -le $area.Bottom) { return }
            # The control may be in the criteria list or in the result pane ("Remove
            # record" is in the history table). Scrolling the wrong pane does nothing.
            if (($r.X + $r.Width / 2) -ge $area.Left) { Point-At-List } else { Point-At-Result $area.PointerY }
            Press-Key "{PGDN}"
        }
    }
    function Invoke-Element($el) {
        Bring-Into-View $el
        $done = $false
        foreach ($pattern in @([System.Windows.Automation.InvokePattern]::Pattern, [System.Windows.Automation.TogglePattern]::Pattern)) {
            if ($done) { break }
            try {
                $action = $el.GetCurrentPattern($pattern)
                if ($pattern -eq [System.Windows.Automation.InvokePattern]::Pattern) { $action.Invoke() } else { $action.Toggle() }
                $done = $true
            } catch {}
        }
        # A pressed-state button offers Toggle instead of Invoke; both are tried above.
        if (-not $done) {
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
    # A raw click at a point, used only where no control is exposed to automation
    # but a real click still proves the behaviour (the criterion title is plain text).
    function Click-Point([int]$x, [int]$y) {
        Focus-App
        [ClassiVarSmokeWin]::SetCursorPos($x, $y) | Out-Null
        [ClassiVarSmokeWin]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
        [ClassiVarSmokeWin]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 450
    }
    # A menu item (the strength menu, the lab menu) may not be exposed as a button,
    # so it is found by its name alone, newest first, since an open menu is drawn last.
    function Choose-Menu-Item([string]$name) {
        Focus-App
        for ($i = 0; $i -lt 25; $i++) {
            $cond = New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::NameProperty, $name)
            $all = @((Root).FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond))
            if ($all.Count -gt 0) { Invoke-Element $all[-1]; return }
            Start-Sleep -Milliseconds 150
        }
        throw "menu item not found: $name"
    }
    # Pick a strength from a criterion's strength menu.
    function Choose-Strength([string]$code, [string]$strength) {
        Scroll-To-Button "$code strength:*"
        Click-Button-Like "$code strength:*"
        Choose-Menu-Item $strength
    }
    function Click-Button-Like([string]$name) {
        Focus-App
        Invoke-Element (Find-Button-Like $name)
    }
    # A text box is found by its accessible name, which its aria label sets
    # (the placeholder text may differ from that name).
    function Find-Box([string]$placeholder) {
        $cond = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::NameProperty, $placeholder)
        # A box that a click just revealed can take a moment to reach the automation tree.
        for ($i = 0; $i -lt 10; $i++) {
            $box = (Root).FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
            if ($null -ne $box) { return $box }
            Start-Sleep -Milliseconds 150
        }
        return $null
    }
    function Gene-Box { return Find-Box "Gene symbol" }
    function Set-GeneText([string]$text) { Set-BoxText "Gene symbol" $text }
    function Press-Key([string]$keys) {
        Focus-App
        [System.Windows.Forms.SendKeys]::SendWait($keys)
        Start-Sleep -Milliseconds 450
    }
    function Press-Escape { Press-Key "{ESC}" }
    function Set-BoxText([string]$placeholder, [string]$text) {
        Focus-App
        $box = Find-Box $placeholder
        # Only what is on screen is in the automation tree. The case form sits at
        # the top of the editor sheet, so a helper's box may be a page further down.
        for ($i = 0; $null -eq $box -and $i -lt 4; $i++) {
            Point-At-List
            Press-Key "{PGDN}"
            $box = Find-Box $placeholder
        }
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
    # A button outside the visible part of the criteria list (below it, or hidden
    # under the pane header) ignores a click. Like a person would, scroll the list
    # (Page Up or Page Down with the pointer over it) until the button is fully on screen.
    function Scroll-To-Button([string]$name) {
        for ($i = 0; $i -lt 10; $i++) {
            $r = (Find-Button-Like $name).Current.BoundingRectangle
            $area = Get-ListArea
            if ($r.Y -ge $area.Top -and ($r.Y + $r.Height) -le $area.Bottom) { return }
            Point-At-List
            if ($r.Y -lt $area.Top) { Press-Key "{PGUP}" } else { Press-Key "{PGDN}" }
        }
        throw "could not scroll the button into view: $name"
    }
    # Put the criteria list back at the top, so the next steps start from a known
    # place. The first thing in the list is the predictor helper's button (the helper
    # must be closed); the list is at the top when that button is below the header.
    function Scroll-List-To-Top {
        for ($i = 0; $i -lt 12; $i++) {
            $r = (Find-Button "Open the predictor score helper").Current.BoundingRectangle
            if ($r.Y -ge (Get-ListArea).Top) { return }
            Point-At-List
            Press-Key "{PGUP}"
        }
        throw "could not scroll the criteria list to the top"
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
        # A background probe: the result pane's empty bottom-right corner, away
        # from result content, criteria rows and scroll bars.
        $c = $bmp.GetPixel(60, $r.Bottom - 40)
        $g.Dispose(); $bmp.Dispose()
        $script:shots += $path
        return [pscustomobject]@{ Path = $path; Background = $c }
    }
    function Names {
        return @((Root).FindAll([System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition) | ForEach-Object { $_.Current.Name })
    }

    Focus-App
    # The first frame can be captured before the window has painted, so wait
    # for a control to appear in the accessibility tree first (it is pushed
    # with the first painted frame), then capture and check the background.
    Click-Button "Start new variant"
    $null = Find-Button-Like "Save case*"
    $light = Capture "01-light"
    if ($light.Background.R -lt 200) { throw "the app did not start in the light theme" }

    Click-Button-Like "Theme:*Switch to dark*"
    $dark = Capture "02-dark"
    if ($dark.Background.R -gt 80) { throw "the background did not change from light to dark" }
    $saved = (Get-Content $settings -Raw).Trim()
    if ($saved -ne "theme=dark") { throw "the theme choice was not saved (settings: $saved)" }
    Find-Button-Like "Theme: dark. Switch to light.*" | Out-Null

    Click-Button "Evidence helpers"
    Set-GeneText "brc"
    $suggest = Capture "03-suggestions"
    Find-Button "BRCA1" | Out-Null
    Find-Button "BRCA2" | Out-Null
    Click-Button "BRCA1"
    $value = (Gene-Box).GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value
    if ($value -ne "BRCA1") { throw "the suggestion did not fill the gene box (got '$value')" }

    # PVS1 asks the gene-level question (Abou Tayoun 2018, Table 1) first, then which situation fits.
    # Open the PVS1 guide. After the guide closes, the list keeps its scroll and the
    # PVS1 row can sit above the visible part, where a click does nothing; scroll to
    # it first. A double click toggles the guide closed again; retry once so a stray
    # repeat from the automation does not fail the run.
    function Open-Pvs1-Guide {
        Scroll-To-Button "PVS1:*not applied*"
        Click-Button-Like "PVS1:*not applied*"
        Start-Sleep -Milliseconds 300
        if (@(Names | Where-Object { $_ -like "PVS1 gene level:*" }).Count -eq 0) {
            Click-Button-Like "PVS1:*not applied*"
        }
    }
    Open-Pvs1-Guide
    if (@(Names | Where-Object { $_ -like "PVS1 Very strong:*" }).Count -ne 0) {
        throw "the PVS1 situations showed before the gene-level question was answered"
    }
    Click-Button "PVS1 gene level: LoF mechanism established"
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
    Open-Pvs1-Guide
    Click-Button-Like "PVS1 gene level: LoF mechanism established"
    Find-Button-Like "PVS1 Very strong:*" | Out-Null
    # The first option is visible without scrolling; later ones may be below the fold.
    Click-Button-Like "PVS1 Very strong:*"
    Find-Button-Like "PVS1:*applied*" | Out-Null
    Find-Button-Like "PVS1 strength: Very strong*" | Out-Null
    $after = Capture "05-pvs1-applied"

    # The lower part of the result: how the points add up, and the 2015 table.
    # They are drawn, not controls, so this is a capture for the eye.
    Point-At-Result 600
    Press-Key "{PGDN}"
    Press-Key "{PGDN}"
    Capture "05k-result-lower" | Out-Null
    # One page further down: "What would change it", with its outlined bars on the scale.
    Press-Key "{PGDN}"
    Capture "05l-what-if" | Out-Null
    for ($i = 0; $i -lt 4; $i++) { Press-Key "{PGUP}" }

    # The predictor helper turns a REVEL score into PP3 at the published strength.
    Scroll-List-To-Top
    Click-Button "Open the predictor score helper"
    Set-BoxText "Predictor score" "0.95"
    Click-Button-Like "Apply PP3 Strong*"
    Find-Button-Like "PP3:*applied*" | Out-Null
    Find-Button-Like "PP3 strength: Strong*" | Out-Null
    # A score between the limits gives no button, and a benign score above
    # this program's limit is lowered to Moderate.
    Set-BoxText "Predictor score" "0.5"
    if (@(Names | Where-Object { $_ -like "Apply PP3*" -or $_ -like "Apply BP4*" }).Count -ne 0) {
        throw "a score between the limits still offered to apply a criterion"
    }
    Set-BoxText "Predictor score" "0.01"
    Find-Button-Like "Apply BP4 Moderate*" | Out-Null
    $predictor = Capture "05b-predictor"
    # SpliceAI is applied at Supporting only; 0.1 to 0.2 gives nothing (Walker 2023).
    # The paper's table is only for variants outside donor/acceptor +/-1,2, so the
    # helper withholds Apply until the curator confirms that, and it accepts 0 to 1.
    Click-Button "SpliceAI"
    Set-BoxText "Predictor score" "0.9"
    if (@(Names | Where-Object { $_ -like "Apply PP3*" -or $_ -like "Apply BP4*" }).Count -ne 0) {
        throw "SpliceAI offered Apply before its scope was confirmed"
    }
    Scroll-To-Button "Confirm the variant is outside*"
    Click-Button-Like "Confirm the variant is outside*"
    Set-BoxText "Predictor score" "0.9"
    Find-Button-Like "Apply PP3 Supporting*" | Out-Null
    Set-BoxText "Predictor score" "0.15"
    if (@(Names | Where-Object { $_ -like "Apply PP3*" -or $_ -like "Apply BP4*" }).Count -ne 0) {
        throw "a SpliceAI score between 0.1 and 0.2 still offered to apply a criterion"
    }
    Set-BoxText "Predictor score" "1.5"
    if (@(Names | Where-Object { $_ -like "Apply PP3*" -or $_ -like "Apply BP4*" }).Count -ne 0) {
        throw "a SpliceAI score outside 0 to 1 still offered to apply a criterion"
    }
    Set-BoxText "Predictor score" "0.05"
    Find-Button-Like "Apply BP4 Supporting*" | Out-Null
    Capture "05b2-spliceai" | Out-Null
    Click-Button "Close the predictor score helper"

    # The functional helper: controls route, then the OddsPath route.
    Scroll-To-Button "Open the functional evidence helper"
    Click-Button "Open the functional evidence helper"
    Click-Button "Confirm the assay has normal and abnormal experimental controls and replicates"
    Set-BoxText "Pathogenic validation control count" "6"
    Set-BoxText "Benign validation control count" "6"
    Click-Button-Like "Apply PS3 Moderate*"
    Find-Button-Like "PS3:*applied*" | Out-Null
    Scroll-To-Button "OddsPath value"
    Click-Button "OddsPath value"
    Set-BoxText "OddsPath value from the assay" "400"
    Find-Button-Like "Apply PS3 Very strong*" | Out-Null
    Set-BoxText "OddsPath value from the assay" "1"
    if (@(Names | Where-Object { $_ -like "Apply PS3*" -or $_ -like "Apply BS3*" }).Count -ne 0) {
        throw "an OddsPath in the indeterminate range still offered to apply a criterion"
    }
    $functional = Capture "05c-functional"
    Click-Button "Close the functional evidence helper"

    # The family and phenotype helper: a yield of 60% earns 3.5 PP4 points and
    # three dominant co-segregations earn 3 PP1 points. The sum, 6.5, is cut to
    # the limit of 5, where the paper allows Strong with Supporting.
    $yieldBox = "Diagnostic yield, percent"
    $countBox = "Informative family member count"
    $variantsBox = "Variants sharing the allele"
    $anyApply = { @(Names | Where-Object { $_ -like "Apply PP1*" -or $_ -like "Apply PP4*" }).Count -ne 0 }
    Scroll-To-Button "Open the co-segregation and phenotype helper"
    Click-Button "Open the co-segregation and phenotype helper"
    Set-BoxText $yieldBox "60"
    Set-BoxText $countBox "3"
    # PP1 is not counted until the curator says whether other genes can cause the phenotype.
    if (& $anyApply) { throw "PP1 was offered before the question about other genes was answered" }
    # Only one gene causes the phenotype: PP1 is not counted, and only PP4 choices are listed.
    Click-Button "Only this gene causes it"
    Find-Button-Like "Apply PP4 Moderate*" | Out-Null
    if (@(Names | Where-Object { $_ -like "Apply PP1*" }).Count -ne 0) {
        throw "PP1 was offered although only one gene causes the phenotype"
    }
    # Several genes: the pairs of Table 4 appear.
    Click-Button "Several genes can cause it"
    # Plain text is not exposed to automation, so the total (5, "cut to the
    # limit") is read from the screenshot; the two choice buttons are checked here.
    Find-Button-Like "Apply PP1 Supporting + PP4 Strong*" | Out-Null
    Scroll-To-Button "Apply PP1 Strong + PP4 Supporting*"
    $locus = Capture "05e-pp1-pp4"
    Click-Button-Like "Apply PP1 Strong + PP4 Supporting*"
    Find-Button-Like "PP1 strength: Strong*" | Out-Null
    Find-Button-Like "PP4 strength: Supporting*" | Out-Null
    # Variants sharing an allele: the yield is divided (95.8% / 2 = 47.9%, 2.5 points),
    # and PP1 is refused because the paper's sharing worksheet is not built in.
    Set-BoxText $variantsBox "2"
    if (& $anyApply) { throw "PP1 was offered for variants that share an allele" }
    Set-BoxText $countBox ""
    Set-BoxText $yieldBox "95.8"
    Find-Button-Like "Apply PP4 Moderate*" | Out-Null
    Set-BoxText $variantsBox ""
    # A yield below the table, or a number that is not a percentage, offers nothing.
    Set-BoxText $yieldBox "10"
    if (& $anyApply) { throw "a yield below the table still offered to apply PP4" }
    Set-BoxText $yieldBox "150"
    if (& $anyApply) { throw "a yield above 100% still offered to apply PP4" }
    Click-Button "Close the co-segregation and phenotype helper"

    # The non-segregation helper offers BS4 only where the paper supports it.
    Scroll-To-Button "Open the non-segregation helper"
    Click-Button "Open the non-segregation helper"
    foreach ($label in @("Recessive, compound heterozygous", "Reduced penetrance, unaffected carriers", "Many look-alike cases (phenocopy)", "Conflicting results or a very large family")) {
        Click-Button $label
        if (@(Names | Where-Object { $_ -like "Apply BS4*" }).Count -ne 0) {
            throw "BS4 was offered for a situation where the paper does not support it: $label"
        }
    }
    Click-Button "Does not follow the disease"
    Scroll-To-Button "Apply BS4 Strong*"
    $segregation = Capture "05f-bs4"
    Click-Button-Like "Apply BS4 Strong*"
    Find-Button-Like "BS4 strength: Strong*" | Out-Null
    Click-Button "Close the non-segregation helper"

    # The de novo helper: two patients with a highly specific phenotype and confirmed
    # parents earn 2 + 2 = 4 points, Very Strong (ClinGen SVI v1.1, Table 2). A
    # recessive condition with no second variant lowers it one level, to Strong.
    # Every count box has an aria label that names its row; the first row is
    # "highly specific, confirmed".
    $patientsBox = "De novo patients, highly-specific (parents confirmed)"
    Scroll-To-Button "Open the de novo helper"
    Click-Button "Open the de novo helper"
    Set-BoxText $patientsBox "2"
    Find-Button-Like "Apply PS2 Very*" | Out-Null
    # The same total can be applied as PM6, which may also reach Very Strong (SVI Table 2).
    Find-Button-Like "Apply PM6 Very*" | Out-Null
    Scroll-To-Button "Recessive condition, no second variant found"
    $deNovo = Capture "05g-de-novo"
    Click-Button "Recessive condition, no second variant found"
    Find-Button-Like "Apply PS2 Strong*" | Out-Null
    if (@(Names | Where-Object { $_ -like "Apply PS2 Very*" }).Count -ne 0) {
        throw "the recessive rule did not lower the de novo strength"
    }
    Scroll-To-Button "Apply PM6 Strong*"
    Click-Button-Like "Apply PM6 Strong*"
    Find-Button-Like "PM6 strength: Strong*" | Out-Null
    # Several criteria are applied here, so rows of the 2015 table are met and marked.
    Point-At-Result 600
    for ($i = 0; $i -lt 2; $i++) { Press-Key "{PGDN}" }
    Capture "05g2-table-met-rows" | Out-Null
    for ($i = 0; $i -lt 8; $i++) { Press-Key "{PGUP}" }
    # No patients: nothing to apply.
    Set-BoxText $patientsBox ""
    if (@(Names | Where-Object { $_ -like "Apply PS2*" -or $_ -like "Apply PM6*" }).Count -ne 0) {
        throw "the de novo helper offered a strength with no patients"
    }
    Scroll-To-Button "Close the de novo helper"
    Click-Button "Close the de novo helper"
    Scroll-List-To-Top

    # PM3: the source's PAH example gives two points, Strong. Four reaches Very Strong.
    Scroll-To-Button "Open the PM3 patients helper"
    Click-Button "Open the PM3 patients helper"
    Set-BoxText "PM3 count: Pathogenic or likely pathogenic, confirmed in trans" "2"
    if (@(Names | Where-Object { $_ -like "Apply PM3*" }).Count -ne 0) {
        throw "PM3 offered evidence before its conditions were confirmed"
    }
    Scroll-To-Button "I checked the PM3 conditions"
    Click-Button "I checked the PM3 conditions"
    Find-Button-Like "Apply PM3 Strong*" | Out-Null
    Set-BoxText "PM3 count: Pathogenic or likely pathogenic, confirmed in trans" "4"
    Scroll-To-Button "Apply PM3 Very*"
    $pm3 = Capture "05h-pm3"
    Click-Button-Like "Apply PM3 Very*"
    Find-Button-Like "PM3 strength: Very*" | Out-Null
    Set-BoxText "PM3 count: Pathogenic or likely pathogenic, confirmed in trans" ""
    if (@(Names | Where-Object { $_ -like "Apply PM3*" }).Count -ne 0) {
        throw "PM3 offered evidence without patients"
    }
    Set-BoxText "PM3 count: Homozygous (same variant on both copies)" "20"
    Find-Button-Like "Apply PM3 Moderate*" | Out-Null
    Set-BoxText "PM3 count: Homozygous (same variant on both copies)" ""
    Set-BoxText "PM3 count: Uncertain second variant, confirmed in trans" "20"
    Find-Button-Like "Apply PM3 Supporting*" | Out-Null
    # A negative count must poison an otherwise valid reading, not count as 0:
    # two confirmed patients alone would earn Strong, so Apply must vanish.
    Set-BoxText "PM3 count: Pathogenic or likely pathogenic, confirmed in trans" "2"
    Find-Button-Like "Apply PM3 Strong*" | Out-Null
    Set-BoxText "PM3 count: Uncertain second variant, confirmed in trans" "-1"
    if (@(Names | Where-Object { $_ -like "Apply PM3*" }).Count -ne 0) {
        throw "PM3 accepted a negative count"
    }
    Scroll-To-Button "Close the PM3 patients helper"
    Click-Button "Close the PM3 patients helper"
    Scroll-To-Button "Open the BA1 exceptions reference"
    Click-Button "Open the BA1 exceptions reference"
    # Plain text is not exposed to automation, so the entries themselves are
    # only a capture (05i); the engine test pins the nine published ids.
    Scroll-To-Button "Close the BA1 exceptions reference"
    $ba1 = Capture "05i-ba1-exceptions"
    Click-Button "Close the BA1 exceptions reference"
    Scroll-List-To-Top

    # Raising a strength above its default asks for a reason first. PM2 (default
    # Supporting, limit Moderate) is one of the few criteria with room above its default.
    # The helper panels above push these rows down, so each click scrolls to its button first.
    Scroll-To-Button "PM2:*not applied*"
    Click-Button-Like "PM2:*not applied*"
    Choose-Strength "PM2" "Moderate"
    $raise = Capture "05d-raise-reason"
    Set-BoxText "Reason for raising the strength" "Absent in a well-covered, ancestry-matched dataset"
    Scroll-To-Button "Save the reason and raise PM2*"
    Click-Button-Like "Save the reason and raise PM2*"
    Find-Button-Like "PM2 strength: Moderate*" | Out-Null
    # Back to the default needs no reason; raising again asks. Escape then leaves the strength unchanged.
    Choose-Strength "PM2" "Supporting"
    Find-Button-Like "PM2 strength: Supporting*" | Out-Null
    Choose-Strength "PM2" "Moderate"
    Find-Button-Like "Save the reason and raise PM2*" | Out-Null
    Press-Escape
    Find-Button-Like "PM2 strength: Supporting*" | Out-Null
    if (@(Names | Where-Object { $_ -like "Save the reason and raise*" }).Count -ne 0) {
        throw "Escape did not close the reason prompt"
    }
    # Clicking the criterion code removes it, and its open reason prompt goes with it.
    Choose-Strength "PM2" "Moderate"
    Find-Button-Like "Save the reason and raise PM2*" | Out-Null
    Scroll-To-Button "PM2:*applied*"
    Click-Button-Like "PM2:*applied*"
    Find-Button-Like "PM2:*not applied*" | Out-Null
    if (@(Names | Where-Object { $_ -like "Save the reason and raise*" }).Count -ne 0) {
        throw "removing PM2 left its reason prompt open"
    }

    # A criterion is applied by clicking its name as well as its code. The code
    # button stays the accessible control; the title is the pointer's larger target
    # and is plain text, so this clicks the row beside the code.
    Scroll-To-Button "PM1:*not applied*"
    $pm1Row = (Find-Button-Like "PM1:*not applied*").Current.BoundingRectangle
    Click-Point ([int]($pm1Row.X + 150)) ([int]($pm1Row.Y + $pm1Row.Height / 2))
    Find-Button-Like "PM1:*applied*" | Out-Null
    $titleClick = Capture "05j-title-click"
    Scroll-To-Button "PM1:*applied*"
    Click-Button-Like "PM1:*applied*"
    Find-Button-Like "PM1:*not applied*" | Out-Null

    # "More" opens the strength ladder: every step with its points, and the program's limit.
    Scroll-To-Button "More about PM1*"
    Click-Button-Like "More about PM1*"
    Find-Button-Like "Less about PM1*" | Out-Null
    $ladderTop = Capture "05j2-strength-ladder-top"
    Press-Key "{PGDN}"
    $ladder = Capture "05j2-strength-ladder"
    Scroll-To-Button "Less about PM1*"
    Click-Button-Like "Less about PM1*"
    Find-Button-Like "More about PM1*" | Out-Null

    # Two Strong benign criteria on top of the pathogenic evidence meet rows of the
    # 2015 table on both sides. The class is withheld, and the table explains why.
    # These are drawn, not controls, so they are captures for the eye.
    Scroll-To-Button "BS1:*not applied*"
    Click-Button-Like "BS1:*not applied*"
    Scroll-To-Button "BS2:*not applied*"
    Click-Button-Like "BS2:*not applied*"
    Point-At-Result 600
    for ($i = 0; $i -lt 10; $i++) { Press-Key "{PGUP}" }
    Capture "05m-withheld-top" | Out-Null
    Press-Key "{PGDN}"
    Press-Key "{PGDN}"
    Capture "05m-withheld-table" | Out-Null
    for ($i = 0; $i -lt 4; $i++) { Press-Key "{PGUP}" }
    Scroll-To-Button "BS1:*applied*"
    Click-Button-Like "BS1:*applied*"
    Scroll-To-Button "BS2:*applied*"
    Click-Button-Like "BS2:*applied*"
    Find-Button-Like "BS2:*not applied*" | Out-Null
    Find-Button-Like "BS1:*not applied*" | Out-Null

    # Page Down and Page Up scroll the list under the mouse pointer.
    Scroll-To-Button "PS1:*"
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
    Set-BoxText "Lab or source name" "Quest Dx"
    Find-Button-Like "Quest Dx*" | Out-Null
    # Put the criteria list at the top, so the Page Down check below has room to move it.
    $c = (Find-Button-Like "PS1:*").Current.BoundingRectangle
    [ClassiVarSmokeWin]::SetCursorPos([int]($c.X + $c.Width / 2), [int]($c.Y + $c.Height / 2)) | Out-Null
    Press-Key "{PGUP}"
    Press-Key "{PGUP}"
    Press-Key "{PGUP}"
    Click-Button "Remove record"
    Start-Sleep -Milliseconds 300
    if (@(Names | Where-Object { $_ -eq "Yes, continue" }).Count -ne 1) {
        throw "Remove record did not ask for confirmation"
    }
    $modal = Capture "07a-confirm-window"
    # The question owns the keyboard. A helper reads the focused element from
    # UIA; after Tab presses it must stay inside the question (its controls are
    # "Cancel" and "Yes, continue"; the window itself carries no name).
    function Focused-Name {
        $el = [System.Windows.Automation.AutomationElement]::FocusedElement
        if ($null -eq $el) { return "" }
        try { return $el.Current.Name } catch { return "" }
    }
    $allowed = @("", "Cancel", "Yes, continue")
    for ($i = 0; $i -lt 3; $i++) {
        Press-Key "{TAB}"
        $focused = Focused-Name
        if ($allowed -notcontains $focused) {
            throw "the confirmation window let Tab focus '$focused' behind or outside itself"
        }
    }
    # Page Down must not scroll the list behind the confirmation window. The position
    # is read with the window already open and after the Tab presses, so the only
    # thing that can move it between the two readings is a scroll: opening the
    # question and moving focus inside it are already settled by then.
    $c = (Find-Button-Like "PS1:*").Current.BoundingRectangle
    $ps1Top = $c.Y
    [ClassiVarSmokeWin]::SetCursorPos([int]($c.X + $c.Width / 2), [int]($c.Y + $c.Height / 2)) | Out-Null
    Press-Key "{PGDN}"
    $ps1Behind = (Find-Button-Like "PS1:*").Current.BoundingRectangle.Y
    if ($ps1Behind -ne $ps1Top) {
        throw "Page Down scrolled the list behind the confirmation window (PS1 at $ps1Top before, $ps1Behind after)"
    }
    # Escape answers Cancel, and the record is still there.
    Press-Escape
    if (@(Names | Where-Object { $_ -eq "Yes, continue" }).Count -ne 0) {
        throw "Escape did not close the confirmation"
    }
    Find-Button-Like "Quest Dx*" | Out-Null
    Click-Button "Remove record"
    Click-Button "Cancel"
    $reported = Capture "07-reported"

    # The loaded example is fictional, and the gene box is empty on it, so the
    # old "same gene, different gene" guard cannot notice a first gene check:
    # checking a gene on the example asks first. Escape leaves it intact.
    Set-GeneText "brca2"
    # The editor sheet may be scrolled down here, with the gene box above the view.
    Scroll-To-Button "Check gene"
    Click-Button "Check gene"
    if (@(Names | Where-Object { $_ -eq "Yes, continue" }).Count -ne 1) {
        throw "checking a gene on the fictional example did not ask first"
    }
    $guard = Capture "07b-example-gene-guard"
    Press-Escape
    if (@(Names | Where-Object { $_ -eq "Yes, continue" }).Count -ne 0) {
        throw "Escape did not close the gene question"
    }
    if (@(Names | Where-Object { $_ -like "PM2:*applied*" }).Count -ne 1) {
        throw "the gene question on the example lost the example's evidence"
    }
    Set-GeneText ""

    Click-Button "Compare reports"
    # The history table: the criterion and Today columns are frozen, and the
    # records between them scroll sideways. The table is plain text, so these are
    # captures for the eye rather than assertions. This one is at the wide window.
    Point-At-Result 600
    Press-Key "{PGDN}"
    Press-Key "{PGDN}"
    $historyWide = Capture "07c-history"
    for ($i = 0; $i -lt 4; $i++) { Press-Key "{PGUP}" }

    # The 960 x 600 minimum window: the top bar, the case panel, the criteria
    # and the result must all stay reachable.
    [ClassiVarSmokeWin]::SetWindowPos($h, [IntPtr]::Zero, 60, 60, 960, 600, 0x0044) | Out-Null
    Start-Sleep -Milliseconds 900
    $small = Capture "08-small-window"
    Find-Button-Like "Save case*" | Out-Null
    Find-Button-Like "PM2:*applied*" | Out-Null

    Click-Button-Like "Theme:*Switch to light*"
    $back = Capture "08-light-again"
    if ($back.Background.R -lt 200) { throw "the background did not return to light" }

    # The same table at the narrow window: the frozen pair still leaves room for
    # two record columns. The pane is short here, so this takes more page steps.
    Point-At-Result 520
    for ($i = 0; $i -lt 6; $i++) { Press-Key "{PGDN}" }
    $history = Capture "09-history-narrow"

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
