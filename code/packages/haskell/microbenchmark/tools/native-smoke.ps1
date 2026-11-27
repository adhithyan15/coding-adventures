$ErrorActionPreference = 'Stop'
$temporary = Join-Path ([System.IO.Path]::GetTempPath()) ("haskell-microbenchmark-{0}" -f [guid]::NewGuid())
New-Item -ItemType Directory -Path $temporary | Out-Null
try {
  $report = Join-Path $temporary 'report.json'
  cabal build exe:array-microbenchmark
  if ($LASTEXITCODE -ne 0) { throw "native benchmark build exited $LASTEXITCODE" }
  $runner = (cabal list-bin exe:array-microbenchmark).Trim()
  if ($LASTEXITCODE -ne 0) { throw "native executable lookup exited $LASTEXITCODE" }
  $previousRevision = $env:GIT_REVISION
  $previousSha = $env:GITHUB_SHA
  try {
    $env:GIT_REVISION = ''
    $env:GITHUB_SHA = ''
    $lines = & $runner --pairs 40
    if ($LASTEXITCODE -ne 0) { throw "native benchmark exited $LASTEXITCODE" }
  } finally {
    if ($null -eq $previousRevision) { Remove-Item Env:GIT_REVISION -ErrorAction SilentlyContinue }
    else { $env:GIT_REVISION = $previousRevision }
    if ($null -eq $previousSha) { Remove-Item Env:GITHUB_SHA -ErrorAction SilentlyContinue }
    else { $env:GITHUB_SHA = $previousSha }
  }
  $encoding = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText($report, ($lines -join "`n") + "`n", $encoding)
  & $runner --replay $report
  if ($LASTEXITCODE -ne 0) { throw "native report replay exited $LASTEXITCODE" }
  $oversized = Join-Path $temporary 'oversized.json'
  $stream = [System.IO.File]::Create($oversized)
  try { $stream.SetLength(4 * 1024 * 1024 + 1) } finally { $stream.Dispose() }
  & $runner --replay $oversized
  if ($LASTEXITCODE -eq 0) { throw 'replay adapter accepted a report above 4 MiB' }
} finally {
  Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
}
