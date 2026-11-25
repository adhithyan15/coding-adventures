$ErrorActionPreference = 'Stop'
$lines = & dart run bin/array_microbenchmark.dart
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$report = ($lines -join "`n") + "`n"
$encoding = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText((Join-Path (Get-Location) '.dart_tool/native-report.json'), $report, $encoding)
dart run tool/replay_report.dart .dart_tool/native-report.json
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
