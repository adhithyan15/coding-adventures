# mosaic-release-launch.ps1 -- launch a Windows release payload and require it
# to stay up.
#
#   pwsh -NoProfile -ExecutionPolicy Bypass -File code/scripts/mosaic-release-launch.ps1 `
#     -ExePath <exe> -Seconds <N> -Label <name>
#
# The Windows half of the release lanes' launcher pair
# (code/specs/mosaic-release-state-gate-v1.md, "Launching a payload"); the
# macOS half is mosaic-release-launch.sh. The question is the same: did the
# freshly extracted app start and keep running? An app that exits early has
# crashed, or refused to start without its Rust runtime -- Compose's Main.kt
# exits in exactly that case -- and the state it left proves nothing.
#
# The mechanism is the one TaskApp's WinUI gate proved on windows-latest,
# lifted here so every lane shares it instead of pasting it:
#
#   Start-Process -WindowStyle Hidden -PassThru   a GUI exe has no console,
#                                                 and a hidden window still
#                                                 runs the whole startup path
#   HasExited, once a second, for -Seconds        early exit = failure
#   Stop-Process -Force, in `finally`             never leave it running
#
# The environment is the caller's: callers remove MOSAIC_APP_LIBRARY and
# MOSAIC_APP_STATE_PATH from their own process before calling, and both this
# script and the app inherit that.
#
# Exit 0 when the app was still running for the whole window; a thrown error
# (exit 1) naming the app's exit code otherwise.

param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [int]$Seconds = 5,
    [string]$Label = 'payload'
)

$ErrorActionPreference = 'Stop'

# Zero would test nothing: any app is "alive" before it has had a chance to
# crash.
if ($Seconds -lt 1) {
    throw "mosaic-release-launch: -Seconds must be at least 1, got $Seconds"
}
if (!(Test-Path -LiteralPath $ExePath -PathType Leaf)) {
    throw "mosaic-release-launch: $ExePath is not a file"
}
$executable = (Resolve-Path -LiteralPath $ExePath).Path

# Run from the drive root, as the bash launcher runs from `/`: nothing the
# app finds may depend on the checkout being the current directory.
$workingDirectory = [System.IO.Path]::GetPathRoot($executable)

$process = Start-Process -FilePath $executable -WorkingDirectory $workingDirectory `
    -WindowStyle Hidden -PassThru
# Reading Handle now keeps it open, so ExitCode is still readable after the
# process is gone; without it PowerShell can report an empty exit code.
$null = $process.Handle

try {
    for ($elapsed = 1; $elapsed -le $Seconds; $elapsed++) {
        Start-Sleep -Seconds 1
        $process.Refresh()
        if ($process.HasExited) {
            throw "The $Label launch exited with code $($process.ExitCode) after ${elapsed}s, before its ${Seconds}s were up"
        }
    }
    Write-Host "$Label`: still running after ${Seconds}s; stopping it"
}
finally {
    if (!$process.HasExited) {
        Stop-Process -Id $process.Id -Force
        $null = $process.WaitForExit(10000)
    }
}
