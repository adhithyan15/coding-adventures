$ErrorActionPreference = 'Stop'
$package = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$distribution = Join-Path $package 'gradle-build/install/microbenchmark'
$tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
$temp = Join-Path $tempRoot ('java-microbenchmark-consumer-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $temp | Out-Null
try {
    $classpath = Join-Path $distribution 'lib/*'
    & javac -classpath $classpath -d $temp (Join-Path $package 'tests/installed-consumer/Consumer.java')
    if ($LASTEXITCODE -ne 0) { throw 'installed consumer compile failed' }
    & java -classpath ($temp + ';' + $classpath) Consumer
    if ($LASTEXITCODE -ne 0) { throw 'installed consumer execution failed' }
    $driverClass = 'com.codingadventures.microbenchmark.ArrayMicrobenchmark'
    $report = Join-Path $temp 'native-report.json'
    $nativeOutput = & java -classpath $classpath $driverClass --pairs 40 --revision installed-consumer
    if ($LASTEXITCODE -ne 0) { throw 'native CLI failed' }
    [System.IO.File]::WriteAllText($report, (($nativeOutput -join "`n") + "`n"), [System.Text.UTF8Encoding]::new($false))
    & java -classpath $classpath $driverClass --replay $report
    if ($LASTEXITCODE -ne 0) { throw 'report replay failed' }
} finally {
    $resolvedTemp = (Resolve-Path -LiteralPath $temp).Path
    if (-not $resolvedTemp.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) { throw 'refusing to remove a directory outside the system temp root' }
    Remove-Item -LiteralPath $resolvedTemp -Recurse
}
