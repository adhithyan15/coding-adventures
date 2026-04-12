# Changelog

All notable changes to this program will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- `Program.fs` — F# entry point using `[<EntryPoint>]` convention, printing
  `Hello, World!` with literate commentary explaining the fsc → CIL → CLR JIT
  pipeline and F#'s place among functional languages (Haskell, Elixir)
- `hello-world-fsharp.fsproj` — .NET 9 F# console project targeting `net9.0`
  with explicit `<Compile Include="Program.fs" />` for deterministic build order
- `BUILD` — single `dotnet run --disable-build-servers` command; works
  identically on Linux, macOS, and Windows with no platform variants needed
