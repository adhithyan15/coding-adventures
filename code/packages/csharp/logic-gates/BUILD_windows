# Run the suite in Debug AND Release: `dotnet pack` ships Release, so the
# optimized build must be tested too.  The 95% line-coverage gate runs on the
# Release pass (the shipped configuration).
dotnet test tests/CodingAdventures.LogicGates.Tests/CodingAdventures.LogicGates.Tests.csproj --disable-build-servers -c Debug
dotnet test tests/CodingAdventures.LogicGates.Tests/CodingAdventures.LogicGates.Tests.csproj --disable-build-servers -c Release /p:CollectCoverage=true /p:Threshold=95 /p:ThresholdType=line
