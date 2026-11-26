Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self
$d = $self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\iso-harness'))) {
    $d = Split-Path -Parent $d
}
if (-not $d) { throw 'iso-harness not found' }
$harness = Join-Path $d 'code\packages\c\iso-harness'
$env:ISO_INCLUDE = "include ..\rank-statistics\include ..\bounded-json\include $harness\include"
. (Join-Path $harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang cpp -Name microbenchmark-tests -Sources @('tests\microbenchmark_test.cpp', 'src\microbenchmark.cpp', 'src\array_workload.cpp')
Iso-BuildAndRun -Lang cpp -Name array-microbenchmark-smoke -Sources @('src\array_driver.cpp', 'src\microbenchmark.cpp', 'src\array_workload.cpp')

$tempRoot = [IO.Path]::GetFullPath($env:TEMP)
$runRoot = Join-Path $tempRoot ("ca-microbenchmark-cpp-" + [guid]::NewGuid().ToString('N'))
$prefix = Join-Path $runRoot 'install'
New-Item -ItemType Directory -Path $runRoot | Out-Null
try {
    $bounded = Join-Path $runRoot 'bounded-json'
    & cmake -S (Join-Path $d 'code\packages\cpp\bounded-json') -B $bounded "-DCMAKE_INSTALL_PREFIX=$prefix" -DCMAKE_BUILD_TYPE=Release
    if ($LASTEXITCODE -ne 0) { throw 'bounded-json configure failed' }
    & cmake --build $bounded --config Release
    if ($LASTEXITCODE -ne 0) { throw 'bounded-json build failed' }
    & cmake --install $bounded --config Release
    if ($LASTEXITCODE -ne 0) { throw 'bounded-json install failed' }

    $rank = Join-Path $runRoot 'rank-statistics'
    & cmake -S (Join-Path $self '..\rank-statistics') -B $rank "-DCMAKE_INSTALL_PREFIX=$prefix" -DCMAKE_BUILD_TYPE=Release
    if ($LASTEXITCODE -ne 0) { throw 'rank-statistics configure failed' }
    & cmake --build $rank --config Release
    if ($LASTEXITCODE -ne 0) { throw 'rank-statistics build failed' }
    & cmake --install $rank --config Release
    if ($LASTEXITCODE -ne 0) { throw 'rank-statistics install failed' }

    $micro = Join-Path $runRoot 'microbenchmark'
    & cmake -S $self -B $micro "-DCMAKE_INSTALL_PREFIX=$prefix" "-DCMAKE_PREFIX_PATH=$prefix" -DCMAKE_BUILD_TYPE=Release
    if ($LASTEXITCODE -ne 0) { throw 'microbenchmark configure failed' }
    & cmake --build $micro --config Release
    if ($LASTEXITCODE -ne 0) { throw 'microbenchmark build failed' }
    & ctest --test-dir $micro -C Release --output-on-failure
    if ($LASTEXITCODE -ne 0) { throw 'microbenchmark tests failed' }
    & cmake --install $micro --config Release
    if ($LASTEXITCODE -ne 0) { throw 'microbenchmark install failed' }

    $consumer = Join-Path $runRoot 'consumer'
    & cmake -S (Join-Path $self 'tests\consumer') -B $consumer "-DCMAKE_PREFIX_PATH=$prefix" -DCMAKE_BUILD_TYPE=Release
    if ($LASTEXITCODE -ne 0) { throw 'consumer configure failed' }
    & cmake --build $consumer --config Release
    if ($LASTEXITCODE -ne 0) { throw 'consumer build failed' }
    $exe = Join-Path $consumer 'ca_microbenchmark_consumer.exe'
    if (-not (Test-Path $exe)) { $exe = Join-Path $consumer 'Release\ca_microbenchmark_consumer.exe' }
    & $exe
    if ($LASTEXITCODE -ne 0) { throw 'installed consumer failed' }
}
finally {
    $resolved = [IO.Path]::GetFullPath($runRoot)
    if (-not [string]::Equals([IO.Path]::GetDirectoryName($resolved), $tempRoot.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase)) { throw 'refusing to remove path outside TEMP' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
