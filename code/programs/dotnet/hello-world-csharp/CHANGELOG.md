# Changelog

All notable changes to this program will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- `Program.cs` — top-level C# 9 entry point printing `Hello, World!` with
  literate commentary explaining the Roslyn → CIL → CLR JIT → CPU pipeline
- `hello-world-csharp.csproj` — .NET 9 console project targeting `net9.0`
  with `Nullable` and `ImplicitUsings` enabled
- `BUILD` — single `dotnet run --disable-build-servers` command; works
  identically on Linux, macOS, and Windows with no platform variants needed
