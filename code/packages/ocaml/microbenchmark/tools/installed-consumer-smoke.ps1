$ErrorActionPreference = 'Stop'
$temporary = Join-Path ([System.IO.Path]::GetTempPath()) ("ocaml-consumer-{0}" -f [guid]::NewGuid())
New-Item -ItemType Directory -Path $temporary | Out-Null
try {
  opam install . --yes --with-test --with-dev-setup
  if ($LASTEXITCODE -ne 0) { throw "opam package install exited $LASTEXITCODE" }
  $encoding = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText((Join-Path $temporary 'dune-project'), "(lang dune 3.16)`n(name benchmark_consumer_smoke)`n", $encoding)
  [System.IO.File]::WriteAllText((Join-Path $temporary 'dune'), "(executable`n (name main)`n (libraries coding-adventures-microbenchmark))`n", $encoding)
  $source = @'
open Microbenchmark

let () =
  match analyze [ { baseline_ns = 100L; candidate_ns = 100L; order = Baseline_first } ] 95 1.05 with
  | Ok result when result.ratio = 1. -> print_endline "installed OCaml consumer passed"
  | _ -> failwith "installed OCaml consumer returned unexpected analysis"
'@
  [System.IO.File]::WriteAllText((Join-Path $temporary 'main.ml'), $source, [System.Text.UTF8Encoding]::new($false))
  opam exec -- dune exec --root $temporary ./main.exe
  if ($LASTEXITCODE -ne 0) { throw "installed consumer exited $LASTEXITCODE" }
  $report = Join-Path $temporary 'native-report.json'
  $previousRevision = $env:GIT_REVISION
  $previousSha = $env:GITHUB_SHA
  try {
    $env:GIT_REVISION = ''
    $env:GITHUB_SHA = ''
    $lines = opam exec -- coding-adventures-array-microbenchmark --pairs 40
    if ($LASTEXITCODE -ne 0) { throw "installed native command exited $LASTEXITCODE" }
  } finally {
    if ($null -eq $previousRevision) { Remove-Item Env:GIT_REVISION -ErrorAction SilentlyContinue }
    else { $env:GIT_REVISION = $previousRevision }
    if ($null -eq $previousSha) { Remove-Item Env:GITHUB_SHA -ErrorAction SilentlyContinue }
    else { $env:GITHUB_SHA = $previousSha }
  }
  [System.IO.File]::WriteAllText($report, ($lines -join "`n") + "`n", [System.Text.UTF8Encoding]::new($false))
  opam exec -- coding-adventures-array-microbenchmark --replay $report
  if ($LASTEXITCODE -ne 0) { throw "installed report replay exited $LASTEXITCODE" }
  $oversized = Join-Path $temporary 'oversized-report.json'
  $stream = [System.IO.File]::Create($oversized)
  try { $stream.SetLength(4 * 1024 * 1024 + 1) } finally { $stream.Dispose() }
  opam exec -- coding-adventures-array-microbenchmark --replay $oversized
  if ($LASTEXITCODE -eq 0) { throw 'replay adapter accepted a report above 4 MiB' }
} finally {
  Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
}
