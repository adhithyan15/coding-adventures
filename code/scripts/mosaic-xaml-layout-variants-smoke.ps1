<#
.SYNOPSIS
    Resize a generated WinUI window across a layout threshold and require the
    root it shows to change with it (UI48 ENV3, §7.11).

.DESCRIPTION
    Launches the native-complete build of
    code/packages/rust/mosaic-emit-xaml/fixtures/layout-variants, which has a
    default layout and a `compact` one selected by the conventional rule
    (`sizeClass` = `compact`, under 600 effective pixels), running against the
    conformance runtime. Each layout shows a marker text naming itself and the
    runtime's `status` and `platform` props.

    The window is resized wide, narrow, wide and narrow again, from outside
    the process. After each resize the script waits for the matching marker,
    requires the other one to be gone, and requires the runtime's props to be
    on screen -- so a frozen layout, a switch that never happens, and a new
    root mounted without its props each fail. A gate that rendered at one
    size only would pass against a frozen layout; this one changes the
    environment, as UI48 §7 asks.

    UI Automation reads the window without a foreground desktop session, as
    the TaskApp smokes do; the resize is plain Win32 SetWindowPos.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [int]$TimeoutSeconds = 30
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -Namespace MosaicSmoke -Name Win32 -MemberDefinition @'
[DllImport("user32.dll", SetLastError = true)]
public static extern bool SetWindowPos(
    System.IntPtr hWnd, System.IntPtr hWndInsertAfter,
    int x, int y, int cx, int cy, uint flags);
[DllImport("user32.dll")]
public static extern uint GetDpiForWindow(System.IntPtr hWnd);
'@

if (-not (Test-Path -LiteralPath $ExePath -PathType Leaf)) {
    throw "Layout-variants fixture executable not found at $ExePath"
}

$defaultMarker = 'Layout: default'
$compactMarker = 'Layout: compact'

# The window's visible text, read while the app may be swapping one root for
# another: an element found a moment ago can leave the tree before its
# properties are read (ElementNotAvailableException, which PowerShell may
# hand over wrapped), and the search itself can race a removal. Either is a
# reading taken mid-swap, not a failure, so it is skipped and the caller
# polls again; only the caller's deadline fails, with what it last saw.
function Get-VisibleText($root) {
    $textCondition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Text)
    $values = @()
    try {
        $elements = $root.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            $textCondition)
    }
    catch {
        return $values
    }
    foreach ($element in $elements) {
        try {
            if (-not $element.Current.IsOffscreen -and $element.Current.Name) {
                $values += $element.Current.Name
            }
        }
        catch {
            continue
        }
    }
    return $values
}

# Resize the window to $width x 800 DEVICE-INDEPENDENT pixels (scaled by the
# window's DPI, so the effective width WinUI lays out at is $width minus the
# frame), then wait until $expected is showing and $absent is not, with the
# runtime's props on screen beside it.
function Resize-AndExpect($process, $root, [int]$width, [string]$expected, [string]$absent) {
    $scale = [MosaicSmoke.Win32]::GetDpiForWindow($process.MainWindowHandle) / 96.0
    if ($scale -le 0) { $scale = 1.0 }
    # SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE
    $flags = 0x0002 -bor 0x0004 -bor 0x0010
    $resized = [MosaicSmoke.Win32]::SetWindowPos(
        $process.MainWindowHandle, [IntPtr]::Zero, 0, 0,
        [int]($width * $scale), [int](800 * $scale), $flags)
    if (-not $resized) {
        throw "SetWindowPos to $width px failed with Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())."
    }
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $visible = @()
    while ((Get-Date) -lt $deadline) {
        $process.Refresh()
        if ($process.HasExited) {
            throw "The fixture exited after a resize to $width px, with code $($process.ExitCode)."
        }
        $visible = @(Get-VisibleText $root)
        if (($expected -in $visible) -and ($absent -notin $visible) -and
            ('windows' -in $visible) -and (@($visible | Where-Object { $_ -in @('started', 'restored') }).Count -gt 0)) {
            Write-Host "At $width px the window shows '$expected' with the runtime's props."
            return
        }
        Start-Sleep -Milliseconds 250
    }
    throw "At $width px expected '$expected' (and not '$absent') with the runtime's status and platform; visible text: $($visible -join ' | ')"
}

$process = $null
try {
    Write-Host "Launching $ExePath"
    $process = Start-Process -FilePath $ExePath -PassThru
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 250
        $process.Refresh()
        if ($process.HasExited) {
            throw "The fixture exited during startup with code $($process.ExitCode)."
        }
        if ($process.MainWindowHandle -ne [IntPtr]::Zero) { break }
    }
    $process.Refresh()
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) {
        throw "The fixture never produced a window within $TimeoutSeconds seconds."
    }
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)

    # Wide, narrow, wide, narrow: each crossing must swap the root, both ways.
    Resize-AndExpect $process $root 1300 $defaultMarker $compactMarker
    Resize-AndExpect $process $root 420 $compactMarker $defaultMarker
    Resize-AndExpect $process $root 1300 $defaultMarker $compactMarker
    Resize-AndExpect $process $root 420 $compactMarker $defaultMarker

    $statusLine = @(Get-VisibleText $root | Where-Object { $_ -like 'Status: Mosaic layout*' })
    if ($statusLine.Count -gt 0) {
        throw "A layout switch reported a failure: $($statusLine -join ' | ')"
    }
    Write-Host 'Layout variants passed: the window swapped roots across 600 px in both directions.'
}
finally {
    if ($process) {
        $process.Refresh()
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
        }
    }
}
