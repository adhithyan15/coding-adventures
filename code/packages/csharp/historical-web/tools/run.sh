#!/bin/sh
set -eu
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
TEST=tests/CodingAdventures.HistoricalWeb.Tests/CodingAdventures.HistoricalWeb.Tests.csproj
FILTER='/p:Include=[CodingAdventures.HistoricalWeb.CSharp]*'
dotnet test "$TEST" --disable-build-servers /p:CollectCoverage=true "$FILTER" /p:Threshold=95 /p:ThresholdType=line
