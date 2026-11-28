Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self
$test = 'tests/CodingAdventures.HistoricalWeb.Tests/CodingAdventures.HistoricalWeb.Tests.csproj'
$include = '/p:Include=[CodingAdventures.HistoricalWeb.CSharp]*'
dotnet test $test --disable-build-servers /p:CollectCoverage=true $include /p:Threshold=95 /p:ThresholdType=line
if ($LASTEXITCODE -ne 0) { throw "Historical Web tests or 95% line coverage gate failed ($LASTEXITCODE)" }
