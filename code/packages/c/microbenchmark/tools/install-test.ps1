$ErrorActionPreference='Stop'
$self=Split-Path -Parent $PSScriptRoot
$cRoot=(Resolve-Path (Join-Path $self '..')).Path
$repo=(Resolve-Path (Join-Path $self '..\..\..\..')).Path
$tempRoot=Join-Path ([IO.Path]::GetTempPath()) ('ca-c-microbenchmark-' + [guid]::NewGuid().ToString('N'))
$prefix=Join-Path $tempRoot 'prefix'
$configuration='Release'
New-Item -ItemType Directory -Path $tempRoot | Out-Null
function Build-Install([string]$Name,[string]$Source,[string[]]$ExtraArgs) {
  $build=Join-Path $tempRoot ($Name + '-build')
  & cmake -S $Source -B $build ("-DCMAKE_INSTALL_PREFIX=$prefix") "-DCMAKE_BUILD_TYPE=$configuration" @ExtraArgs
  if ($LASTEXITCODE -ne 0) { throw "$Name configure failed" }
  & cmake --build $build --config $configuration
  if ($LASTEXITCODE -ne 0) { throw "$Name build failed" }
  & cmake --install $build --config $configuration
  if ($LASTEXITCODE -ne 0) { throw "$Name install failed" }
}
function Invoke-NativeCapture([string]$Executable,[string[]]$Arguments) {
  $quotedArguments = foreach ($argument in $Arguments) {
    if ($argument.Contains('"')) { throw 'native test argument contains an unsupported quote' }
    '"' + $argument + '"'
  }
  $startInfo = New-Object System.Diagnostics.ProcessStartInfo
  $startInfo.FileName = $Executable
  $startInfo.Arguments = $quotedArguments -join ' '
  $startInfo.UseShellExecute = $false
  $startInfo.CreateNoWindow = $true
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true
  $process = New-Object System.Diagnostics.Process
  $process.StartInfo = $startInfo
  try {
    if (-not $process.Start()) { throw 'could not start native negative-case process' }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    return [pscustomobject]@{
      ExitCode = $process.ExitCode
      Stdout = $stdoutTask.GetAwaiter().GetResult()
      Stderr = $stderrTask.GetAwaiter().GetResult()
    }
  } finally {
    $process.Dispose()
  }
}
try {
  Build-Install 'bounded-json' (Join-Path $repo 'code\packages\c\bounded-json') @()
  Build-Install 'rank-statistics' (Join-Path $cRoot 'rank-statistics') @()
  Build-Install 'microbenchmark' $self @("-DCMAKE_PREFIX_PATH=$prefix")
  $consumerBuild=Join-Path $tempRoot 'consumer-build'
  & cmake -S (Join-Path $self 'tests\install') -B $consumerBuild ("-DCMAKE_PREFIX_PATH=$prefix") "-DCMAKE_BUILD_TYPE=$configuration"
  if ($LASTEXITCODE -ne 0) { throw 'installed consumer configure failed' }
  & cmake --build $consumerBuild --config $configuration
  if ($LASTEXITCODE -ne 0) { throw 'installed consumer build failed' }
  $consumerExe=Join-Path (Join-Path $consumerBuild $configuration) 'installed-consumer.exe'
  if (-not (Test-Path -LiteralPath $consumerExe)) {
    $consumerExe=Join-Path $consumerBuild 'installed-consumer.exe'
  }
  & $consumerExe
  if ($LASTEXITCODE -ne 0) { throw 'installed consumer execution failed' }
  $driver=Join-Path $prefix 'bin\array-microbenchmark.exe'
  $reportPath=Join-Path $tempRoot 'native-report.json'
  $json=& $driver --revision install-smoke --pairs 40
  if ($LASTEXITCODE -ne 0) { throw 'installed 40-pair CLI failed' }
  [IO.File]::WriteAllText($reportPath,[string]$json,[Text.UTF8Encoding]::new($false))
  & $driver --replay $reportPath
  if ($LASTEXITCODE -ne 0) { throw 'installed native CLI report replay failed' }
  $missingReport=Join-Path $tempRoot 'missing-report.json'
  $malformedReport=Join-Path $tempRoot 'malformed-report.json'
  [IO.File]::WriteAllText($malformedReport,'{',[Text.UTF8Encoding]::new($false))
  $oversizedReport=Join-Path $tempRoot 'oversized-report.json'
  $oversizedStream=[IO.File]::Open($oversizedReport,[IO.FileMode]::Create,[IO.FileAccess]::Write,[IO.FileShare]::None)
  try { $oversizedStream.SetLength(4194305) } finally { $oversizedStream.Dispose() }
  $negativeCases = @(
    @{ Name='missing replay'; Path=$missingReport },
    @{ Name='malformed replay'; Path=$malformedReport },
    @{ Name='oversized replay'; Path=$oversizedReport }
  )
  foreach ($case in $negativeCases) {
    $captured = Invoke-NativeCapture -Executable $driver -Arguments @('--replay', $case.Path)
    [IO.File]::WriteAllText((Join-Path $tempRoot 'replay-output.txt'), $captured.Stdout, [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText((Join-Path $tempRoot 'replay-error.txt'), $captured.Stderr, [Text.UTF8Encoding]::new($false))
    if ($captured.ExitCode -eq 0) { throw "installed native CLI accepted $($case.Name) input" }
    if ($captured.Stdout.Length -ne 0) { throw "$($case.Name) emitted stdout" }
    if ([string]::IsNullOrWhiteSpace($captured.Stderr)) { throw "$($case.Name) emitted no diagnostic" }
  }
  $global:LASTEXITCODE=0
  Write-Host 'installed consumer passed'
} finally {
  $resolvedTemp=[IO.Path]::GetFullPath($tempRoot)
  $resolvedBase=[IO.Path]::GetFullPath([IO.Path]::GetTempPath())
  if ($resolvedTemp.StartsWith($resolvedBase,[StringComparison]::OrdinalIgnoreCase) -and (Test-Path -LiteralPath $resolvedTemp)) { Remove-Item -LiteralPath $resolvedTemp -Recurse }
}
