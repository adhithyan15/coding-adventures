$ErrorActionPreference = 'Stop'
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("dotnet-mb-csharp-" + [guid]::NewGuid().ToString('N'))
$feed = Join-Path $work 'feed'
$packages = Join-Path $work 'packages'
$config = Join-Path $work 'NuGet.Config'
New-Item -ItemType Directory -Path $feed, $packages | Out-Null
$escapedFeed = [System.Security.SecurityElement]::Escape($feed)
$configText = @"
<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <packageSources>
    <clear />
    <add key="local" value="$escapedFeed" />
    <add key="nuget.org" value="https://api.nuget.org/v3/index.json" />
  </packageSources>
</configuration>
"@
[System.IO.File]::WriteAllText($config, $configText, [System.Text.UTF8Encoding]::new($false))
try {
    dotnet pack ../rank-statistics/CodingAdventures.RankStatistics.CSharp.csproj -o $feed --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'rank-statistics pack failed' }
    dotnet pack CodingAdventures.Microbenchmark.CSharp.csproj -o $feed --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'microbenchmark pack failed' }
    dotnet restore consumer/Consumer.csproj --configfile $config --packages $packages --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'installed consumer restore failed' }
    dotnet run --project consumer/Consumer.csproj --no-restore --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'installed consumer execution failed' }
}
finally {
    Remove-Item -LiteralPath $work -Recurse -Force
}
