<#
.SYNOPSIS
    Launch the generated WinUI HostButton-selected fixture and assert the
    selected state reaches UI Automation (UI86, #15463).

.DESCRIPTION
    The XAML emitter lowers `HostButton ( selected : ... )` to a generated
    Button subclass whose automation peer adds the SelectionItem pattern. The
    emitter tests only prove the markup; this proves the running control:

      * a button with `selected : true` exposes SelectionItem, IsSelected true;
      * a button bound to a false slot exposes SelectionItem, IsSelected false;
      * a button without `selected` does not expose SelectionItem at all
        (absent is not false, UI86 section 3.2);
      * every one of them is still a Button with the Invoke pattern, so
        activation and the existing smoke paths are unchanged.

.PARAMETER ExePath
    Path to the built Picker.exe from
    code/packages/rust/mosaic-emit-xaml/fixtures/host-selected-button.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [int]$TimeoutSeconds = 30,
    # Also check the For rows: `option` buttons bound to `i == selectedIndex`
    # with selectedIndex = 1, and `from-binding` buttons bound to flags
    # [true, false]; then invoke option 0 and expect the selection to move.
    # Needs props the fixture's generated host does not supply on its own
    # (it has no engine), so it is for a locally patched build.
    [switch]$WithRows
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

if (-not (Test-Path $ExePath)) {
    Write-Error "Picker executable not found at $ExePath"
    exit 1
}

function Find-Button($root, $automationId) {
    $cond = New-Object System.Windows.Automation.AndCondition(
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::AutomationIdProperty, $automationId)),
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Button)))
    return $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
}

function Find-Buttons($root, $automationId) {
    $cond = New-Object System.Windows.Automation.AndCondition(
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::AutomationIdProperty, $automationId)),
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Button)))
    return @($root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond))
}

function Get-States($root, $automationId) {
    return (Find-Buttons $root $automationId | ForEach-Object { Get-Selection $_ }) -join ','
}

function Get-Selection($element) {
    $pattern = $null
    if ($element.TryGetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern, [ref]$pattern)) {
        return [bool]$pattern.Current.IsSelected
    }
    return $null
}

function Test-Invokable($element) {
    $pattern = $null
    return $element.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)
}

$proc = $null
$failures = @()
try {
    Write-Host "Launching $ExePath"
    $proc = Start-Process -FilePath $ExePath -PassThru
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $proc.Refresh()
        if ($proc.HasExited) {
            throw "Picker exited during startup with code $($proc.ExitCode)."
        }
        if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { break }
    }
    $proc.Refresh()
    if ($proc.MainWindowHandle -eq [IntPtr]::Zero) {
        throw "Picker never produced a window within $TimeoutSeconds seconds."
    }
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)

    $expected = @(
        @{ Id = 'raised';    Selection = $true },
        @{ Id = 'from-slot'; Selection = $false },
        @{ Id = 'plain';     Selection = $null }
    )
    foreach ($case in $expected) {
        $element = $null
        $waitUntil = (Get-Date).AddSeconds($TimeoutSeconds)
        while ((Get-Date) -lt $waitUntil -and -not $element) {
            $element = Find-Button $root $case.Id
            if (-not $element) { Start-Sleep -Milliseconds 250 }
        }
        if (-not $element) {
            $failures += "No Button with AutomationId '$($case.Id)'."
            continue
        }
        $actual = Get-Selection $element
        $shown = if ($null -eq $actual) { 'no SelectionItem pattern' } else { "IsSelected=$actual" }
        Write-Host "  $($case.Id): $shown"
        if ($null -eq $case.Selection) {
            if ($null -ne $actual) {
                $failures += "'$($case.Id)' has no selected prop but exposes SelectionItem ($shown)."
            }
        } elseif ($actual -ne $case.Selection) {
            $failures += "'$($case.Id)' expected IsSelected=$($case.Selection), got $shown."
        }
        if (-not (Test-Invokable $element)) {
            $failures += "'$($case.Id)' lost the Invoke pattern."
        }
    }
    if ($WithRows) {
        $pairs = @(
            @{ Id = 'from-binding'; Want = 'True,False' },
            @{ Id = 'option'; Want = 'False,True,False' }
        )
        foreach ($pair in $pairs) {
            $got = Get-States $root $pair.Id
            Write-Host "  $($pair.Id) rows: $got"
            if ($got -ne $pair.Want) { $failures += "'$($pair.Id)' rows expected $($pair.Want), got $got." }
        }
        $first = (Find-Buttons $root 'option')[0]
        $pattern = $first.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $pattern.Invoke()
        $want = 'True,False,False'
        $got = ''
        $waitUntil = (Get-Date).AddSeconds(10)
        while ((Get-Date) -lt $waitUntil) {
            Start-Sleep -Milliseconds 250
            $got = Get-States $root 'option'
            if ($got -eq $want) { break }
        }
        Write-Host "  option rows after invoking row 0: $got"
        if ($got -ne $want) { $failures += "selection did not move to row 0: $got." }
        # A screen reader's own Select action dispatches the same event.
        $last = (Find-Buttons $root 'option')[2]
        $last.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select()
        $want = 'False,False,True'
        $waitUntil = (Get-Date).AddSeconds(10)
        while ((Get-Date) -lt $waitUntil) {
            Start-Sleep -Milliseconds 250
            $got = Get-States $root 'option'
            if ($got -eq $want) { break }
        }
        Write-Host "  option rows after SelectionItem.Select on row 2: $got"
        if ($got -ne $want) { $failures += "SelectionItem.Select did not move the selection to row 2: $got." }
    }
}
finally {
    if ($proc -and -not $proc.HasExited) {
        Stop-Process -Id $proc.Id -Force
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Host "FAIL: $_" }
    exit 1
}
Write-Host 'Selected-button UI Automation smoke passed.'
exit 0
