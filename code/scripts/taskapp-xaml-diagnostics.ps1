# Collect only known build outputs. Diagnostics are never release candidates.
param(
    [Parameter(Mandatory = $true)][string]$Commit,
    [string]$Assets = 'release-assets',
    [string]$SmokeLog = 'release-work/xaml-smoke.log',
    [string]$Output = 'xaml-failure-diagnostics'
)
$ErrorActionPreference = 'Stop'
# A fresh directory prevents stale or unrelated files from entering the upload.
New-Item -ItemType Directory -Path $Output -ErrorAction Stop | Out-Null
$payloads = @()
foreach ($kind in @('bundle', 'project')) {
    if (!(Test-Path -LiteralPath $Assets)) { continue }
    foreach ($file in Get-ChildItem -LiteralPath $Assets -File -Filter "task-app-xaml-windows-$kind-v*.zip") {
        if ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) { continue }
        Copy-Item -LiteralPath $file.FullName -Destination $Output
        $payloads += @{
            name = $file.Name
            sha256 = (Get-FileHash -LiteralPath (Join-Path $Output $file.Name) -Algorithm SHA256).Hash
        }
    }
}
# Smoke tests use synthetic data. Bound both line count and individual line size.
if (Test-Path -LiteralPath $SmokeLog -PathType Leaf) {
    Get-Content -LiteralPath $SmokeLog -Tail 200 |
        ForEach-Object { $_.Substring(0, [Math]::Min($_.Length, 4096)) } |
        Set-Content -LiteralPath (Join-Path $Output 'smoke-tail.log') -Encoding utf8
}
@{ source_commit = $Commit; verified = $false; payloads = @($payloads) } |
    ConvertTo-Json -Depth 4 |
    Set-Content -LiteralPath (Join-Path $Output 'receipt.json') -Encoding utf8
