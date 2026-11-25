$ErrorActionPreference = 'Stop'
$feed = Join-Path ([System.IO.Path]::GetTempPath()) ("dotnet-mb-fsharp-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $feed | Out-Null
try {
    dotnet pack ../rank-statistics/CodingAdventures.RankStatistics.FSharp.fsproj -o $feed --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'rank-statistics pack failed' }
    dotnet pack CodingAdventures.Microbenchmark.FSharp.fsproj -o $feed --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'microbenchmark pack failed' }
    dotnet restore consumer/Consumer.fsproj -s $feed -s 'https://api.nuget.org/v3/index.json' --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'installed consumer restore failed' }
    dotnet run --project consumer/Consumer.fsproj --no-restore --verbosity quiet
    if ($LASTEXITCODE -ne 0) { throw 'installed consumer execution failed' }
}
finally {
    Remove-Item -LiteralPath $feed -Recurse -Force
}
