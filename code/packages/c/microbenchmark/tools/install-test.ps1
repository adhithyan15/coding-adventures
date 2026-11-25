$ErrorActionPreference='Stop'
$self=Split-Path -Parent $PSScriptRoot
$cRoot=(Resolve-Path (Join-Path $self '..')).Path
$repo=(Resolve-Path (Join-Path $self '..\..\..\..')).Path
$tempRoot=Join-Path ([IO.Path]::GetTempPath()) ('ca-c-microbenchmark-' + [guid]::NewGuid().ToString('N'))
$prefix=Join-Path $tempRoot 'prefix'
New-Item -ItemType Directory -Path $tempRoot | Out-Null
function Build-Install([string]$Name,[string]$Source,[string[]]$ExtraArgs) {
  $build=Join-Path $tempRoot ($Name + '-build')
  & cmake -S $Source -B $build ("-DCMAKE_INSTALL_PREFIX=$prefix") @ExtraArgs
  if ($LASTEXITCODE -ne 0) { throw "$Name configure failed" }
  & cmake --build $build
  if ($LASTEXITCODE -ne 0) { throw "$Name build failed" }
  & cmake --install $build
  if ($LASTEXITCODE -ne 0) { throw "$Name install failed" }
}
try {
  Build-Install 'bounded-json' (Join-Path $repo 'code\packages\c\bounded-json') @()
  Build-Install 'rank-statistics' (Join-Path $cRoot 'rank-statistics') @()
  Build-Install 'microbenchmark' $self @("-DCMAKE_PREFIX_PATH=$prefix")
  $consumerBuild=Join-Path $tempRoot 'consumer-build'
  & cmake -S (Join-Path $self 'tests\install') -B $consumerBuild ("-DCMAKE_PREFIX_PATH=$prefix")
  if ($LASTEXITCODE -ne 0) { throw 'installed consumer configure failed' }
  & cmake --build $consumerBuild
  if ($LASTEXITCODE -ne 0) { throw 'installed consumer build failed' }
  & (Join-Path $consumerBuild 'installed-consumer.exe')
  if ($LASTEXITCODE -ne 0) { throw 'installed consumer execution failed' }
  $driver=Join-Path $prefix 'bin\array-microbenchmark.exe'
  $reportPath=Join-Path $tempRoot 'native-report.json'
  $json=& $driver --revision install-smoke --pairs 40
  if ($LASTEXITCODE -ne 0) { throw 'installed 40-pair CLI failed' }
  [IO.File]::WriteAllText($reportPath,[string]$json,[Text.UTF8Encoding]::new($false))
  & $driver --replay $reportPath
  if ($LASTEXITCODE -ne 0) { throw 'installed native CLI report replay failed' }
  Write-Host 'installed consumer passed'
} finally {
  $resolvedTemp=[IO.Path]::GetFullPath($tempRoot)
  $resolvedBase=[IO.Path]::GetFullPath([IO.Path]::GetTempPath())
  if ($resolvedTemp.StartsWith($resolvedBase,[StringComparison]::OrdinalIgnoreCase) -and (Test-Path -LiteralPath $resolvedTemp)) { Remove-Item -LiteralPath $resolvedTemp -Recurse }
}
