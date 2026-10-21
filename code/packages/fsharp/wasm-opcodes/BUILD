# --artifacts-path gives every project in the reference graph its own
# .artifacts/obj/<project>/ and .artifacts/bin/<project>/ directory.
# Do not pass /p:BaseIntermediateOutputPath=...\$(MSBuildProjectName)/ instead:
# MSBuild never expands property references inside a command-line /p: value,
# so every project would share one literal "$(MSBuildProjectName)" obj dir,
# one project.assets.json and one AssemblyAttributes file, and the build
# fails depending on ordering (CS0579 duplicate attributes, or missing
# xunit references after the assets file is overwritten).
mkdir -p .dotnet .artifacts && DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1 DOTNET_CLI_HOME="$PWD/.dotnet" dotnet test tests/CodingAdventures.WasmOpcodes.Tests/CodingAdventures.WasmOpcodes.Tests.fsproj --disable-build-servers --artifacts-path .artifacts /p:CollectCoverage=true /p:Threshold=80 /p:ThresholdType=line
