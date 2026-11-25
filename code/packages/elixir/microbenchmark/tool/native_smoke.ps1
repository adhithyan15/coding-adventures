$ErrorActionPreference = 'Stop'
$report = Join-Path $env:TEMP ("elixir-microbenchmark-{0}.json" -f [guid]::NewGuid())
try {
  & $env:ComSpec /d /c "escript .\coding_adventures_microbenchmark > `"$report`""
  if ($LASTEXITCODE -ne 0) { throw "native benchmark exited $LASTEXITCODE" }
  mix run tool/native_report_replay.exs $report
  if ($LASTEXITCODE -ne 0) { throw "native report replay exited $LASTEXITCODE" }
} finally {
  Remove-Item -LiteralPath $report -ErrorAction SilentlyContinue
}
