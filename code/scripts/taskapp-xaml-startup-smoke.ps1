<#
.SYNOPSIS
    Prove that the WinUI TaskApp reports a missing runtime and can retry in place.

.DESCRIPTION
    Temporarily removes the packaged Mosaic runtime, launches the real TaskApp,
    and checks its startup-failure UI through Windows UI Automation. The script
    then restores the runtime, invokes Try again, and requires the normal task
    input to appear without restarting the process.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [Parameter(Mandatory = $true)][string]$RuntimePath,
    [int]$TimeoutSeconds = 30
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

if (-not (Test-Path -LiteralPath $ExePath -PathType Leaf)) {
    throw "TaskApp executable not found at $ExePath"
}
if (-not (Test-Path -LiteralPath $RuntimePath -PathType Leaf)) {
    throw "TaskApp runtime not found at $RuntimePath"
}

$runtimeBackup = "$RuntimePath.startup-acceptance"
if (Test-Path -LiteralPath $runtimeBackup) {
    throw "Refusing to overwrite existing startup-acceptance backup at $runtimeBackup"
}

$process = $null
$runtimeMoved = $false

function Find-ByAutomationId($root, $automationId, $controlType = $null) {
    $idCondition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty,
        $automationId)
    $condition = $idCondition
    if ($null -ne $controlType) {
        $condition = New-Object System.Windows.Automation.AndCondition(
            $idCondition,
            (New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                $controlType)))
    }
    return $root.FindFirst(
        [System.Windows.Automation.TreeScope]::Descendants,
        $condition)
}

function Get-VisibleText($root) {
    $textCondition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Text)
    $values = @()
    foreach ($element in $root.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            $textCondition)) {
        if (-not $element.Current.IsOffscreen -and $element.Current.Name) {
            $values += $element.Current.Name
        }
    }
    return $values
}

function Wait-ForElement($root, $automationId, $controlType, $timeoutSeconds) {
    $deadline = (Get-Date).AddSeconds($timeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $element = Find-ByAutomationId $root $automationId $controlType
        if ($element -and -not $element.Current.IsOffscreen) {
            return $element
        }
        Start-Sleep -Milliseconds 250
    }
    return $null
}

try {
    Move-Item -LiteralPath $RuntimePath -Destination $runtimeBackup
    $runtimeMoved = $true

    Write-Host "Launching TaskApp without $RuntimePath"
    $process = Start-Process -FilePath $ExePath -PassThru
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 250
        $process.Refresh()
        if ($process.HasExited) {
            throw "TaskApp exited during recoverable startup with code $($process.ExitCode)."
        }
        if ($process.MainWindowHandle -ne [IntPtr]::Zero) { break }
    }
    $process.Refresh()
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) {
        throw "TaskApp never produced a startup window within $TimeoutSeconds seconds."
    }

    $root = [System.Windows.Automation.AutomationElement]::FromHandle(
        $process.MainWindowHandle)
    # WinUI does not expose a layout-only Grid in UI Automation's control
    # view, even when it carries an AutomationId. The source contract pins the
    # grid's mosaic-startup-failure id; use its selectable detail control as
    # the live-tree proof that the failure surface is actually visible.
    $failureDetail = Wait-ForElement `
        $root `
        'mosaic-startup-failure-detail' `
        ([System.Windows.Automation.ControlType]::Edit) `
        $TimeoutSeconds
    if (-not $failureDetail) {
        throw 'The startup failure did not expose selectable loader detail.'
    }
    $detail = $failureDetail.GetCurrentPattern(
        [System.Windows.Automation.ValuePattern]::Pattern).Current.Value
    if ($detail -notlike '*native-complete requires the Mosaic Rust application runtime*' -or
        $detail -notlike '*Loader detail:*') {
        throw "Startup failure omitted actionable loader detail: $detail"
    }
    $visibleText = Get-VisibleText $root
    if ('TaskApp could not start' -notin $visibleText) {
        throw "Startup failure heading was not visible. Text: $($visibleText -join ' | ')"
    }
    if ('Your saved tasks have not been changed. Retrying is safe.' -notin $visibleText) {
        throw "Startup failure did not reassure users about saved tasks. Text: $($visibleText -join ' | ')"
    }

    $retry = Wait-ForElement `
        $root `
        'mosaic-startup-retry' `
        ([System.Windows.Automation.ControlType]::Button) `
        $TimeoutSeconds
    if (-not $retry) {
        throw 'The startup failure did not expose mosaic-startup-retry.'
    }

    Move-Item -LiteralPath $runtimeBackup -Destination $RuntimePath
    $runtimeMoved = $false
    $retry.GetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern).Invoke()

    $nameInput = Wait-ForElement `
        $root `
        'name-input' `
        ([System.Windows.Automation.ControlType]::Edit) `
        $TimeoutSeconds
    $process.Refresh()
    if ($process.HasExited) {
        throw "TaskApp exited while retrying startup with code $($process.ExitCode)."
    }
    if (-not $nameInput) {
        throw 'TaskApp did not reveal the normal name-input after the runtime was restored.'
    }
    Write-Host 'Recoverable startup passed: failure was visible and retry reached TaskApp.'
}
finally {
    if ($process) {
        $process.Refresh()
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
        }
    }
    if ($runtimeMoved -and (Test-Path -LiteralPath $runtimeBackup)) {
        Move-Item -LiteralPath $runtimeBackup -Destination $RuntimePath
    }
}
