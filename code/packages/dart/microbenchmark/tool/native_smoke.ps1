$ErrorActionPreference = 'Stop'
$previousRevision = $env:GIT_REVISION
try {
  $env:GIT_REVISION = ''
  $lines = & dart run bin/array_microbenchmark.dart
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  if ($null -eq $previousRevision) { Remove-Item Env:GIT_REVISION -ErrorAction SilentlyContinue }
  else { $env:GIT_REVISION = $previousRevision }
}
$report = ($lines -join "`n") + "`n"
$encoding = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText((Join-Path (Get-Location) '.dart_tool/native-report.json'), $report, $encoding)
dart run tool/replay_report.dart .dart_tool/native-report.json
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$oversized = Join-Path (Get-Location) '.dart_tool/oversized-report.bin'
[System.IO.File]::WriteAllBytes($oversized, [byte[]]::new(4 * 1024 * 1024 + 1))
dart run tool/replay_report.dart $oversized
if ($LASTEXITCODE -eq 0) { throw 'replay adapter accepted a report above 4 MiB' }
