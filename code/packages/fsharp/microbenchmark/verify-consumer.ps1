$ErrorActionPreference = 'Stop'
$tempRoot = [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath ([System.IO.Path]::GetTempPath())).ProviderPath).TrimEnd([char[]]@('\', '/'))
$runId = [guid]::NewGuid().ToString('N')
$workName = "dotnet-mb-fsharp-$runId"
$work = Join-Path $tempRoot $workName
$feed = Join-Path $work 'feed'
$packages = Join-Path $work 'packages'
$config = Join-Path $work 'NuGet.Config'
$workCreated = $false
try {
    New-Item -ItemType Directory -Path $work -ErrorAction Stop | Out-Null
    $workCreated = $true
    New-Item -ItemType Directory -Path $feed, $packages -ErrorAction Stop | Out-Null
    $escapedFeed = [System.Security.SecurityElement]::Escape($feed)
    $configText = @"
<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <packageSources>
    <clear />
    <add key="local" value="$escapedFeed" />
    <add key="nuget.org" value="https://api.nuget.org/v3/index.json" />
  </packageSources>
  <packageSourceMapping>
    <packageSource key="local">
      <package pattern="CodingAdventures.*" />
    </packageSource>
    <packageSource key="nuget.org">
      <package pattern="*" />
    </packageSource>
  </packageSourceMapping>
</configuration>
"@
    [System.IO.File]::WriteAllText($config, $configText, [System.Text.UTF8Encoding]::new($false))
    dotnet pack ../rank-statistics/CodingAdventures.RankStatistics.FSharp.fsproj -o $feed --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'rank-statistics pack failed' }
    dotnet pack CodingAdventures.Microbenchmark.FSharp.fsproj -o $feed --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'microbenchmark pack failed' }
    dotnet restore consumer/Consumer.fsproj --configfile $config --packages $packages --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'installed consumer restore failed' }
    dotnet run --project consumer/Consumer.fsproj --no-restore --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'installed consumer execution failed' }
}
finally {
    if ($workCreated -and (Test-Path -LiteralPath $work)) {
        $tempPrefix = $tempRoot + [System.IO.Path]::DirectorySeparatorChar
        $workItem = Get-Item -LiteralPath $work -Force
        if ($workItem -isnot [System.IO.DirectoryInfo] -or ($workItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw 'refusing to remove an unexpected or reparse-point temporary directory'
        }
        $resolvedWork = [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $work).ProviderPath)
        $expectedWork = [System.IO.Path]::GetFullPath((Join-Path $tempRoot $workName))
        if (-not $resolvedWork.StartsWith($tempPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or
            $resolvedWork -ne $expectedWork -or $resolvedWork -eq $tempRoot) {
            throw 'refusing to remove a temporary directory outside its expected GUID path'
        }
        Remove-Item -LiteralPath $resolvedWork -Recurse -Force
    }
}
