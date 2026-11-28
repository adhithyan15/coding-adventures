#!/bin/sh
set -eu
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
ADAPTER=tests/ParityAdapter/CodingAdventures.HistoricalWeb.ParityAdapter.csproj
DOTNET_EXE=${DOTNET:-dotnet}
PYTHON_EXE=${PYTHON:-python3}
"$DOTNET_EXE" build "$ADAPTER" --disable-build-servers --configuration Release
"$PYTHON_EXE" tests/CodingAdventures.HistoricalWeb.Tests/python_parity.py \
    "$DOTNET_EXE" tests/ParityAdapter/bin/Release/net9.0/CodingAdventures.HistoricalWeb.ParityAdapter.dll
