dotnet test tests/CodingAdventures.Microbenchmark.FSharp.Tests/CodingAdventures.Microbenchmark.FSharp.Tests.fsproj --disable-build-servers /p:CollectCoverage=true /p:Threshold=90 /p:ThresholdType=line
dotnet run --project cli/CodingAdventures.ArrayMicrobenchmark.FSharp.fsproj -- --pairs 40 --revision fsharp-build-smoke
./verify-consumer.ps1
