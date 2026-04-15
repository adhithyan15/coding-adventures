# .NET Hello World — C# and F#

## Overview

This spec covers adding .NET language support to the build tool and shipping two
hello-world programs: one in C# and one in F#. Both programs are pure console
applications that print `Hello, World!` to stdout and serve as the entry point
for the computing-stack journey in the .NET ecosystem.

### Why a single `dotnet` language key?

C# and F# are distinct languages but share an identical toolchain:

- Both compile to **CIL (Common Intermediate Language)** bytecode
- Both are built and run with the **`dotnet` CLI**
- Both use **NuGet** for package management
- Both produce the same output structure (`obj/`, `bin/`)

From the build tool's perspective, the language is the toolchain, not the syntax.
A `dotnet` language key covers both C# (`.csproj`) and F# (`.fsproj`) packages.
This is the same reasoning that makes `typescript` cover both `.ts` and `.tsx` files.

---

## Platform Support

The `dotnet` CLI is a first-class cross-platform tool. The same command works on
Linux, macOS, and Windows with no modification:

```bash
dotnet run --disable-build-servers
```

**No platform-specific BUILD files are needed.** A single `BUILD` file is correct
for all three platforms — unlike Lua or some Perl tooling that requires
`BUILD_windows` variants.

### `--disable-build-servers` is mandatory in BUILD files

The `dotnet` CLI starts a long-lived **MSBuild server** process in the background
to speed up subsequent builds. In CI this causes two problems:

1. **Port conflicts** — The second build step (`--force` full rebuild) tries to
   start another MSBuild server on the same port. It fails with "address already
   in use."
2. **Zombie processes** — The server continues running after the build step
   exits, consuming memory and potentially interfering with parallel matrix jobs.

`--disable-build-servers` suppresses the server entirely. The build is slightly
slower (no warm server) but correct and safe for parallel CI execution.

This flag is explicitly documented by Microsoft as the recommended approach for
CI pipelines.

---

## Directory Structure

All .NET packages live under a `dotnet/` path component:

```
code/programs/dotnet/
  hello-world-csharp/
    BUILD                    ← dotnet run --disable-build-servers
    hello-world-csharp.csproj
    Program.cs
    README.md
    CHANGELOG.md
  hello-world-fsharp/
    BUILD                    ← dotnet run --disable-build-servers
    hello-world-fsharp.fsproj
    Program.fs
    README.md
    CHANGELOG.md
```

The `dotnet/` directory component is what the build tool's `inferLanguage()`
function recognises as the `dotnet` language — the same mechanism used for
`python/`, `ruby/`, `go/`, etc.

### Project file naming convention

The `.csproj` or `.fsproj` file **must match the directory name exactly**
(e.g., `hello-world-csharp.csproj` in directory `hello-world-csharp/`).
The .NET SDK uses the project filename as the default assembly name.

---

## Target Framework

All .NET programs and packages in this repo target **`net9.0`** — the current
long-term support (LTS) release. This aligns with what `actions/setup-dotnet@v4`
installs in CI when `dotnet-version: '9.0'` is specified.

---

## C# Project File

Minimal .NET 9 console project:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net9.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
  </PropertyGroup>
</Project>
```

- `Nullable>enable` — enables the nullable reference type system introduced
  in C# 8. This catches null-reference bugs at compile time rather than runtime.
- `ImplicitUsings>enable` — auto-imports common namespaces (`System`,
  `System.Collections.Generic`, etc.) so each file does not need `using`
  directives for the standard library.
- Top-level statements (C# 9+) are used in `Program.cs` — no `class Program`
  or `static void Main` boilerplate required.

## F# Project File

Minimal .NET 9 F# console project:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net9.0</TargetFramework>
  </PropertyGroup>
  <ItemGroup>
    <Compile Include="Program.fs" />
  </ItemGroup>
</Project>
```

F# differs from C# in one important way: **compilation order is significant**.
F# is a top-to-bottom language — a module can only reference symbols defined
above it. The `<Compile Include="...">` entries in the project file define this
order. Even for a single-file project, the entry must be explicit.

`Nullable` and `ImplicitUsings` are C#-only features and are omitted from F#
project files.

The entry point uses the `[<EntryPoint>]` attribute pattern:

```fsharp
[<EntryPoint>]
let main _ =
    printfn "Hello, World!"
    0
```

This is unambiguous across all .NET SDK versions. Top-level F# expressions
work in `.fsx` script files but the `[<EntryPoint>]` convention is the
standard for compiled console applications.

---

## Build Tool Changes Required

### 1. `internal/discovery/discovery.go` — `inferLanguage()`

Add `"dotnet"` to the known-language slice so packages under `code/programs/dotnet/`
or `code/packages/dotnet/` are assigned the `dotnet` language rather than `unknown`.

### 2. `main.go` — `allLanguages`

Add `"dotnet"` to the canonical language list. This ensures:
- `--detect-languages` emits `needs_dotnet=true|false`
- `--force` mode marks dotnet packages as needing rebuild
- The CI detect job outputs the correct flag

### 3. `internal/hasher/hasher.go` — source extensions

Add dotnet entries to both maps so the hasher tracks the right files:

```go
// sourceExtensions
"dotnet": {".cs": true, ".fs": true, ".csproj": true, ".fsproj": true}

// specialFilenames
"dotnet": {"global.json": true, "NuGet.Config": true, "nuget.config": true}
```

`global.json` pins the .NET SDK version — changing it should invalidate the
build cache. `NuGet.Config` / `nuget.config` configures package sources.

### 4. `internal/resolver/resolver.go`

Add `parseDotnetDeps` to read `<ProjectReference>` elements from `.csproj` and
`.fsproj` files, and register `"dotnet"` in both `buildKnownNamesForLanguage`
and the `ResolveDependencies` dispatch switch.

NuGet package names use the directory basename directly (no prefix convention
like Python's `coding-adventures-` prefix).

### 5. `internal/starlark/evaluator.go`

Add `"dotnet_library("` and `"dotnet_binary("` to the `knownRules` slice in
`IsStarlarkBuild()`, and add a corresponding case in `GenerateCommands()`:

```go
case "dotnet_library", "dotnet_binary":
    return []string{
        "dotnet build --disable-build-servers",
        "dotnet test --disable-build-servers",
    }
```

### 6. `detect_languages_test.go`

Add `"dotnet": true` to the `expected` map in `TestAllLanguagesConstant`.
This test guards against accidentally omitting a language from `allLanguages`.

---

## CI Workflow Changes

### detect job — `outputs` block

Add:
```yaml
needs_dotnet: ${{ steps.toolchains.outputs.needs_dotnet }}
```

### detect job — `Normalize toolchain requirements` step

In both the `is_main == 'true'` branch and the else branch, add:
```
'needs_dotnet=true'          # or ${{ steps.detect.outputs.needs_dotnet }}
```

### build job — new conditional step

After the Haskell setup step, add:

```yaml
- name: Set up .NET
  if: needs.detect.outputs.needs_dotnet == 'true'
  uses: actions/setup-dotnet@v4
  with:
    dotnet-version: '9.0'
```

No platform exclusions needed — `actions/setup-dotnet@v4` works on Ubuntu,
macOS, and Windows runners.

---

## NuGet Global Cache — Concurrency Safety

Unlike Gradle/Maven, NuGet's global package cache (`~/.nuget/packages`) is
**content-addressed and write-once**. Multiple concurrent `dotnet restore`
processes can safely read from the same cache. Writes use atomic
temp-file-then-rename operations.

This means dotnet packages do **not** require `--serial-languages dotnet`
or any serialization mechanism. They can build in full parallel alongside
every other language in the monorepo.

The `NUGET_PACKAGES` environment variable can redirect the global cache
for future hermetic sandbox builds, but it is not required for correctness today.

---

## Verification Checklist

Before merging:

- [ ] `go test ./...` passes in `code/programs/go/build-tool/`
- [ ] `TestAllLanguagesConstant` includes `dotnet`
- [ ] `code/programs/dotnet/hello-world-csharp/` is discovered as `dotnet` language
- [ ] `code/programs/dotnet/hello-world-fsharp/` is discovered as `dotnet` language
- [ ] CI detect job emits `needs_dotnet=true` when dotnet packages are in the diff
- [ ] CI build job installs .NET 9 when `needs_dotnet == 'true'`
- [ ] `dotnet run --disable-build-servers` succeeds in both program directories
- [ ] Both programs print `Hello, World!` to stdout
