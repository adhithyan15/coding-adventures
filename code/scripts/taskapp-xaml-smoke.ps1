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
Add-Type @'
using System;
using System.Runtime.InteropServices;

public static class TaskAppWindow {
    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool MoveWindow(
        IntPtr hWnd,
        int x,
        int y,
        int width,
        int height,
        bool repaint);
}
'@

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

function Find-ByNameAndAutomationId($root, $name, $automationId, $controlType) {
    $cond = New-Object System.Windows.Automation.AndCondition(
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::NameProperty, $name)),
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::AutomationIdProperty, $automationId)),
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

function Set-TaskAppWindowSize($proc, $width, $height) {
    $proc.Refresh()
    if (-not [TaskAppWindow]::MoveWindow(
            $proc.MainWindowHandle, 100, 100, $width, $height, $true)) {
        $errorCode = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "MoveWindow failed for ${width}x${height} with Win32 error $errorCode."
    }
    Start-Sleep -Milliseconds 750
    return [System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)
}

function Find-LiveByAutomationId($root, $automationIds, $controlType) {
    # TaskApp swaps several of its composer inputs behind an `If`/`Else` on
    # focus/validation slots (e.g. name-input / name-input-corrected /
    # name-input-error, due-input / due-input-corrected / due-input-error --
    # see TaskApp.mll). All of those branches can stay in the automation
    # tree at once, collapsed rather than removed, so looking a control up
    # by AutomationId once and reusing that reference is not enough: after
    # the app swaps branches, the cached reference points at a collapsed
    # element that raises "Target element cannot receive focus" from
    # SetFocus(), not ElementNotAvailableException. Return whichever
    # candidate id is currently on-screen.
    foreach ($automationId in $automationIds) {
        $element = Find-ByAutomationId $root $automationId $controlType
        if ($element -and -not $element.Current.IsOffscreen) { return $element }
    }
    return $null
}

function Set-InputFocus($root, $automationIds, $controlType, $timeoutSeconds) {
    # WinUI's generated onChange handler dispatches TextChanged only from a
    # focused TextBox (mosaic-emit-xaml's emit_host_input), which real typing
    # always satisfies but ValuePattern.SetValue does not by itself -- it
    # writes the Text property directly without focusing the control.
    # SetFocus() requests focus, but it is a request, not a guarantee: the
    # app's own auto-focus Loaded handler races it during startup, a
    # packaged app's window can take longer to become the one WinUI's focus
    # manager will actually hand focus to, and (per Find-LiveByAutomationId)
    # which branch is even on-screen can change underneath a cached
    # reference. Re-resolve the live element and retry until it reports
    # HasKeyboardFocus, returning it for the caller to use immediately
    # rather than assuming any single SetFocus() call landed.
    $deadline = (Get-Date).AddSeconds($timeoutSeconds)
    $lastSeenId = '<not found>'
    while ((Get-Date) -lt $deadline) {
        $element = Find-LiveByAutomationId $root $automationIds $controlType
        if ($element) {
            $lastSeenId = $element.Current.AutomationId
            try {
                if ($element.Current.HasKeyboardFocus) { return $element }
                $element.SetFocus()
            } catch [System.Windows.Automation.ElementNotAvailableException] {
            } catch [System.InvalidOperationException] {
                # "Target element cannot receive focus": the branch this id
                # resolved to became collapsed between the IsOffscreen check
                # above and this call. Re-resolve on the next iteration.
            }
        }
        Start-Sleep -Milliseconds 200
    }
    throw "Could not focus any of '$($automationIds -join "', '")' within $timeoutSeconds seconds (last seen: '$lastSeenId')."
}

function Wait-ForNamedOffscreenState($root, $name, $controlType, $expected, $timeoutSeconds) {
    $deadline = (Get-Date).AddSeconds($timeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $element = Find-ByName $root $name $controlType
        if ($element) {
            if ($element.Current.IsOffscreen -eq $expected) { return $true }
        } elseif ($expected) {
            # WinUI may remove closed PaneCustomContent from the control view
            # instead of retaining it as an off-screen element. Both states
            # mean the pane content is unavailable at the compact width.
            return $true
        }
        Start-Sleep -Milliseconds 250
    }
    return $false
}

function Wait-ForAutomationIdOffscreenState(
    $root, $automationId, $controlType, $expected, $timeoutSeconds) {
    $deadline = (Get-Date).AddSeconds($timeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $element = Find-ByAutomationId $root $automationId $controlType
        if ($element -and $element.Current.IsOffscreen -eq $expected) { return $true }
        Start-Sleep -Milliseconds 250
    }
    return $false
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

    # HostNavigationSplit must provide a real, named pane landmark and delegate
    # adaptation to NavigationView's own layout pass. Exercise that behavior on
    # the running app: wide -> narrow -> wide, while the detail stays usable.
    $root = Set-TaskAppWindowSize $proc 1280 900
    $navigationPane = Find-ByNameAndAutomationId $root 'Projects' 'app-shell' ([System.Windows.Automation.ControlType]::Pane)
    if (-not $navigationPane) {
        throw "Could not find the Projects pane landmark with AutomationId app-shell."
    }
    if (-not (Wait-ForNamedOffscreenState $root 'Projects' ([System.Windows.Automation.ControlType]::Text) $false $TimeoutSeconds)) {
        $failures += 'The project pane content was not visible at the wide window size.'
    }
    if (-not (Wait-ForAutomationIdOffscreenState $root 'segmented-option-selected' ([System.Windows.Automation.ControlType]::Button) $false $TimeoutSeconds)) {
        $failures += 'The detail content was not visible at the wide window size.'
    }

    $root = Set-TaskAppWindowSize $proc 520 900
    if (-not (Wait-ForNamedOffscreenState $root 'Projects' ([System.Windows.Automation.ControlType]::Text) $true $TimeoutSeconds)) {
        $failures += 'The project pane content did not collapse at the narrow window size.'
    }
    if (-not (Wait-ForAutomationIdOffscreenState $root 'segmented-option-selected' ([System.Windows.Automation.ControlType]::Button) $false $TimeoutSeconds)) {
        $failures += 'The detail content became unavailable when the project pane collapsed.'
    }

    $root = Set-TaskAppWindowSize $proc 1280 900
    if (-not (Wait-ForNamedOffscreenState $root 'Projects' ([System.Windows.Automation.ControlType]::Text) $false $TimeoutSeconds)) {
        $failures += 'The project pane content did not return after widening the window.'
    } else {
        Write-Host '  adaptive Projects pane collapsed and restored; detail stayed visible'
    }

    # TaskApp once crashed inside Microsoft.UI.Xaml.dll as soon as the Board
    # option was invoked. The ordinary smoke stayed green because it exercised
    # only the initial List view. Enter the Board through the same UI Automation
    # path a user takes, require the selected state to reach the screen, and then
    # return to List before continuing the task-flow assertions below.
    $boardButton = Find-ByNameAndAutomationId $root 'Board' 'segmented-option' ([System.Windows.Automation.ControlType]::Button)
    if (-not $boardButton) {
        throw "Could not find the Board view option. Buttons present: $((Get-ButtonNames $root) -join ', ')"
    }
    $boardButton.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()

    $selectedView = $null
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $proc.Refresh()
        if ($proc.HasExited) {
            throw "TaskApp exited with code $($proc.ExitCode) while opening the Board view."
        }
        $selectedView = Find-ByAutomationId $root 'segmented-option-selected' ([System.Windows.Automation.ControlType]::Button)
        if ($selectedView -and $selectedView.Current.Name -eq 'Board') { break }
    }
    if (-not $selectedView -or $selectedView.Current.Name -ne 'Board') {
        $failures += 'The Board event did not render Board as the selected view.'
    } else {
        Write-Host '  Board view activated without a process exit'
    }

    $listButton = Find-ByNameAndAutomationId $root 'List' 'segmented-option' ([System.Windows.Automation.ControlType]::Button)
    if (-not $listButton) {
        throw 'Could not find the List view option after entering Board.'
    }
    $listButton.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $proc.Refresh()
        if ($proc.HasExited) {
            throw "TaskApp exited with code $($proc.ExitCode) while returning to the List view."
        }
        $selectedView = Find-ByAutomationId $root 'segmented-option-selected' ([System.Windows.Automation.ControlType]::Button)
        if ($selectedView -and $selectedView.Current.Name -eq 'List') {
            break
        }
    }
    if (-not $selectedView -or $selectedView.Current.Name -ne 'List') {
        $failures += 'Returning from Board did not render List as the selected view.'
    }

    # ── 3. A dispatched event changes what is on screen ──────────────────
    #
    # This is the assertion the headless harness cannot make. Props reaching
    # the component object is not the same as the screen updating: with
    # x:Bind defaulting to OneTime, the object was correct and the window was
    # frozen.
    # TaskApp swaps the composer/due-date input behind name-input(-corrected|
    # -error) / due-input(-corrected|-error) branches (see Find-LiveByAutomationId).
    $nameInputIds = 'name-input', 'name-input-corrected', 'name-input-error'
    $dueInputIds = 'due-input', 'due-input-corrected', 'due-input-error'
    $editType = [System.Windows.Automation.ControlType]::Edit
    if (-not (Find-LiveByAutomationId $root $nameInputIds $editType)) {
        throw "Could not find the task composer input. Buttons present: $((Get-ButtonNames $root) -join ', ')"
    }
    $taskName = 'CI smoke task'
    $composer = Set-InputFocus $root $nameInputIds $editType $TimeoutSeconds
    $composer.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($taskName)
    $due = '2026-01-09'
    if (-not (Find-LiveByAutomationId $root $dueInputIds $editType)) {
        throw "Could not find the due-date input."
    }
    $dueInput = Set-InputFocus $root $dueInputIds $editType $TimeoutSeconds
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
    $reopenLabel = "Reopen task: $taskName"
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if ((Get-ButtonNames $root) -contains $reopenLabel) { break }
    }
    if ((Get-ButtonNames $root) -notcontains $reopenLabel) {
        $failures += "Completing the task did not render the completed state."
    }
    $toggle.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    $completeLabel = "Complete task: $taskName"
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if ((Get-ButtonNames $root) -contains $completeLabel) { break }
    }
    if ((Get-ButtonNames $root) -notcontains $completeLabel) {
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
    #
    # The composer and due-date inputs lost focus to the toggle/delete
    # controls exercised above, and adding the first task above can also
    # have swapped which name-input/due-input branch is on-screen (see
    # Find-LiveByAutomationId and Set-InputFocus), so re-resolve them
    # rather than reusing the step 3 references.
    $persistedTask = 'Persisted native task'
    $composer = Set-InputFocus $root $nameInputIds $editType $TimeoutSeconds
    $composer.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($persistedTask)
    $dueInput = Set-InputFocus $root $dueInputIds $editType $TimeoutSeconds
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
