<#
.SYNOPSIS
    Launch the generated WinUI TaskApp and assert the screen reacts to a real event.

.DESCRIPTION
    CI builds the generated XAML app and runs a headless ABI conformance harness
    that asserts on the component *object's* properties. Those assertions pass
    even when the app is completely broken, because props really do reach the
    object -- nothing checks that the screen reflects them, and nothing launches
    the GUI.

    Three separate shipped defects were green under that arrangement:

      1. The app crashed before drawing a pixel (missing app PRI ->
         E_XAMLPARSEFAILED).
      2. The app rendered once and froze (118/153 bindings defaulting to
         x:Bind's OneTime).
      3. Every button inside a For rendered blank (no Content attribute
         emitted at all).

    Each was found by a human launching the app and looking at it. This script
    is that loop, automated. It drives the real window through UI Automation
    and asserts on rendered values, so all three classes fail here rather than
    shipping.

    UI Automation does not require a visible foreground window, so this runs on
    an ordinary hosted runner -- it does not need an interactive desktop
    session.

.PARAMETER ExePath
    Path to the built TaskApp.exe.

.PARAMETER TimeoutSeconds
    How long to wait for the window and for each dispatched event to land.

.PARAMETER RestartExePath
    Optional replacement executable used for the persistence restart. Omitting
    it restarts ExePath, preserving the original single-build acceptance flow.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [string]$RestartExePath = '',
    [int]$TimeoutSeconds = 30
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

if (-not (Test-Path $ExePath)) {
    Write-Error "TaskApp executable not found at $ExePath"
    exit 1
}
$effectiveRestartExePath = if ($RestartExePath) { $RestartExePath } else { $ExePath }
if (-not (Test-Path $effectiveRestartExePath)) {
    Write-Error "TaskApp replacement executable not found at $effectiveRestartExePath"
    exit 1
}

$proc = $null
$failures = @()

function Get-Descendants($root, $controlType) {
    $cond = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty, $controlType)
    return $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
}

function Get-TextValues($root) {
    $out = @()
    foreach ($e in Get-Descendants $root ([System.Windows.Automation.ControlType]::Text)) {
        if ($e.Current.Name) { $out += $e.Current.Name }
    }
    return $out
}

function Get-ButtonNames($root) {
    $out = @()
    foreach ($e in Get-Descendants $root ([System.Windows.Automation.ControlType]::Button)) {
        if ($e.Current.Name) { $out += $e.Current.Name }
    }
    return $out
}

function Find-ByName($root, $name, $controlType) {
    $cond = New-Object System.Windows.Automation.AndCondition(
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::NameProperty, $name)),
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty, $controlType)))
    return $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
}

function Find-ByAutomationId($root, $automationId, $controlType) {
    $cond = New-Object System.Windows.Automation.AndCondition(
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::AutomationIdProperty, $automationId)),
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty, $controlType)))
    return $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
}

try {
    # ── 1. It launches at all ────────────────────────────────────────────
    #
    # A missing app PRI made this fail with E_XAMLPARSEFAILED while the build
    # stayed green, so "the process is still alive" is itself an assertion.
    Write-Host "Launching $ExePath"
    $proc = Start-Process -FilePath $ExePath -PassThru

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $proc.Refresh()
        if ($proc.HasExited) {
            throw "TaskApp exited during startup with code $($proc.ExitCode). It did not render."
        }
        if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { break }
    }
    $proc.Refresh()
    if ($proc.MainWindowHandle -eq [IntPtr]::Zero) {
        throw "TaskApp never produced a window within $TimeoutSeconds seconds."
    }
    Write-Host "  window is up (pid $($proc.Id))"

    $root = [System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)

    # ── 2. The initial render reached the screen ─────────────────────────
    $before = Get-TextValues $root
    $summaryBefore = $before | Where-Object { $_ -like '*task(s)*' } | Select-Object -First 1
    if (-not $summaryBefore) {
        $failures += "No summary text rendered. Visible text: $($before -join ' | ')"
    } else {
        Write-Host "  initial summary: $summaryBefore"
    }

    # ── 3. A dispatched event changes what is on screen ──────────────────
    #
    # This is the assertion the headless harness cannot make. Props reaching
    # the component object is not the same as the screen updating: with
    # x:Bind defaulting to OneTime, the object was correct and the window was
    # frozen.
    $composer = Find-ByAutomationId $root 'name-input' ([System.Windows.Automation.ControlType]::Edit)
    if (-not $composer) {
        throw "Could not find the task composer input. Buttons present: $((Get-ButtonNames $root) -join ', ')"
    }
    $taskName = 'CI smoke task'
    $composer.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($taskName)
    $due = '2026-01-09'
    $dueInput = Find-ByAutomationId $root 'due-input' ([System.Windows.Automation.ControlType]::Edit)
    if (-not $dueInput) {
        throw "Could not find the due-date input."
    }
    $dueInput.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($due)
    Start-Sleep -Seconds 2

    $addButton = Find-ByAutomationId $root 'add-btn' ([System.Windows.Automation.ControlType]::Button)
    if (-not $addButton) {
        throw "Could not find the 'Add task' button."
    }
    $addButton.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()

    $summaryAfter = $null
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $summaryAfter = Get-TextValues $root | Where-Object { $_ -like '*task(s)*' } | Select-Object -First 1
        if ($summaryAfter -and $summaryAfter -ne $summaryBefore) { break }
    }

    if (-not $summaryAfter -or $summaryAfter -eq $summaryBefore) {
        $failures += "Adding a task did not change the rendered summary (still '$summaryBefore'). " +
                     "The engine ran but the UI is frozen -- this is the x:Bind OneTime class of bug."
    } else {
        Write-Host "  summary updated: $summaryAfter"
    }

    # ── 4. The new row actually rendered its content ─────────────────────
    #
    # A HostButton whose label came from a row expression emitted no Content
    # at all, so rows appeared as invisible empty buttons while every other
    # signal looked healthy.
    $buttons = Get-ButtonNames $root
    if ($buttons -notcontains $taskName) {
        $failures += "The new task row did not render its name. Expected a control named '$taskName'. " +
                     "Buttons present: $($buttons -join ', ')"
    } else {
        Write-Host "  task row rendered: $taskName"
    }

    $visible = Get-TextValues $root
    if ($visible -notcontains "due $due") {
        $failures += "The new task row did not render its due date."
    }

    # ── 5. Scheduling detail is reachable through the emitted control ───
    $complexity = Find-ByAutomationId $root 'complexity-toggle' ([System.Windows.Automation.ControlType]::Button)
    if (-not $complexity) {
        throw "Could not find the project-complexity control."
    }
    $complexity.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    $schedule = '2026-01-05 → 2026-01-05'
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if ((Get-TextValues $root) -contains $schedule) { break }
    }
    if ((Get-TextValues $root) -notcontains $schedule) {
        $failures += "Switching to Full CPM did not render the Rust schedule '$schedule'."
    } else {
        Write-Host "  schedule rendered: $schedule"
    }

    # ── 6. Complete, reopen, and delete through emitted row controls ────
    $toggle = Find-ByAutomationId $root 'toggle' ([System.Windows.Automation.ControlType]::Button)
    if (-not $toggle) { throw "Could not find the task completion control." }
    $toggle.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if ((Get-ButtonNames $root) -contains '✓') { break }
    }
    if ((Get-ButtonNames $root) -notcontains '✓') {
        $failures += "Completing the task did not render the completed state."
    }
    $toggle.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if ((Get-ButtonNames $root) -contains '○') { break }
    }
    if ((Get-ButtonNames $root) -notcontains '○') {
        $failures += "Reopening the task did not render the open state."
    }

    $delete = Find-ByAutomationId $root 'del-btn' ([System.Windows.Automation.ControlType]::Button)
    if (-not $delete) { throw "Could not find the task delete control." }
    $delete.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if ((Get-ButtonNames $root) -notcontains $taskName) { break }
    }
    if ((Get-ButtonNames $root) -contains $taskName) {
        $failures += "Deleting the task left its row visible."
    }

    # ── 7. Persist a second task, restart, and prove it is restored ─────
    $persistedTask = 'Persisted native task'
    $composer.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($persistedTask)
    $dueInput.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($due)
    Start-Sleep -Seconds 1
    $addButton.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if ((Get-ButtonNames $root) -contains $persistedTask) { break }
    }
    if ((Get-ButtonNames $root) -notcontains $persistedTask) {
        throw "Could not create the task used for restart persistence."
    }

    Stop-Process -Id $proc.Id -Force
    $proc.WaitForExit()
    $proc = Start-Process -FilePath $effectiveRestartExePath -PassThru
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $proc.Refresh()
        if ($proc.HasExited) {
            throw "TaskApp exited during persisted restart with code $($proc.ExitCode)."
        }
        if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { break }
    }
    if ($proc.MainWindowHandle -eq [IntPtr]::Zero) {
        throw "TaskApp never produced a window after persisted restart."
    }
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if ((Get-ButtonNames $root) -contains $persistedTask) { break }
    }
    if ((Get-ButtonNames $root) -notcontains $persistedTask) {
        $failures += "The persisted task was not restored after restarting the native app."
    } elseif ((Get-TextValues $root) -notcontains $schedule) {
        $failures += "The restored task lost its Rust schedule projection."
    } else {
        Write-Host "  persisted restart restored: $persistedTask"
    }
}
catch {
    $failures += $_.Exception.Message
}
finally {
    if ($proc -and -not $proc.HasExited) {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
}

if ($failures.Count -gt 0) {
    Write-Host ''
    Write-Host 'TaskApp XAML smoke test FAILED:' -ForegroundColor Red
    foreach ($f in $failures) { Write-Host "  - $f" -ForegroundColor Red }
    exit 1
}

Write-Host ''
Write-Host 'TaskApp XAML smoke test passed: native controls completed the scheduled todo lifecycle and restored it after restart.'
exit 0
