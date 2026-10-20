---
category: C#
---

# MSBuild does not expand property references in a command-line /p: value

Since April, the eight C# and eight F# `wasm-*` BUILD files passed
`"/p:BaseIntermediateOutputPath=$PWD/.artifacts/obj/\$(MSBuildProjectName)/"`.
The intent was one obj directory per project. MSBuild treats a global property
from the command line as a literal string, though, so every project in the
test's reference graph (the test project, the library and its three
`ProjectReference`s) wrote to a single directory named
`obj/$(MSBuildProjectName)/`. That meant one `project.assets.json`, one
`nuget.g.props` and one `.NETCoreApp,Version=v9.0.AssemblyAttributes.cs` for
all of them.

The outcome depended on ordering, so it stayed hidden for months. A clean
local build passed. Main CI (run 39331357869) failed with CS0579 duplicate
assembly attributes in `wasm-types` and `wasm-leb128`. A second local run failed
differently: the test project read a library's `project.assets.json` and lost
its xunit references (CS0246 `Fact`).

A literal `$(...)` in an error path is the tell. Use
`dotnet test --artifacts-path .artifacts`, which already gives each project
`.artifacts/obj/<project>/` and `.artifacts/bin/<project>/`. If you must set
a per-project path yourself, set it inside MSBuild (a `Directory.Build.props`
evaluated before the SDK), never through `/p:`. Run the BUILD twice from the
same tree when verifying: a shared-directory bug often passes on a clean
first run.
