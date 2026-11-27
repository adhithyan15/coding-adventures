$ErrorActionPreference = 'Stop'
$report = Join-Path $env:TEMP ("elixir-microbenchmark-{0}.json" -f [guid]::NewGuid())
$oversized = "$report.oversized"
try {
  $previousRevision = $env:GIT_REVISION
  try {
    $env:GIT_REVISION = ''
    & $env:ComSpec /d /c "escript .\coding_adventures_microbenchmark > `"$report`""
    if ($LASTEXITCODE -ne 0) { throw "native benchmark exited $LASTEXITCODE" }
  } finally {
    if ($null -eq $previousRevision) { Remove-Item Env:GIT_REVISION -ErrorAction SilentlyContinue }
    else { $env:GIT_REVISION = $previousRevision }
  }
  mix run tool/native_report_replay.exs $report
  if ($LASTEXITCODE -ne 0) { throw "native report replay exited $LASTEXITCODE" }
  $stream = [System.IO.File]::Create($oversized)
  try { $stream.SetLength(4 * 1024 * 1024 + 1) } finally { $stream.Dispose() }
  mix run tool/native_report_replay.exs $oversized
  if ($LASTEXITCODE -eq 0) { throw 'replay adapter accepted a report above 4 MiB' }
} finally {
  Remove-Item -LiteralPath $report -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $oversized -ErrorAction SilentlyContinue
}
