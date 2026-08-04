# CCPP01 — C & C++ multi-compiler (GCC · Clang/LLVM · MSVC) ISO lane

Status: in progress (PR1 of ~5)

## Motivation

The monorepo builds ~15 languages through the Go build tool
(`code/programs/go/build-tool/`). C and C++ are **not** first-class:

- `inferLanguage` (`internal/discovery/discovery.go`) does not recognize `c` or
  `cpp` as path components, so the existing C++ packages (`cpp/conduit`,
  `cpp/conduit-hello`, `cpp/mosaic-flux-qt`) are inferred as language
  `"unknown"`.
- There is no C/C++ toolchain in `allToolchains` (`main.go`), so CI never
  deliberately installs a C/C++ compiler — those packages merely happen to build
  on the runner's preinstalled `cc`.
- Existing C/C++ build scripts pick **one** compiler (`clang++` else `g++`) and
  **skip Windows/MSVC** entirely. Nothing forces the code to be portable ISO
  C/C++.

**Goal:** make C and C++ first-class in the build tool and CI, and guarantee that
every *pure-ISO* C/C++ package compiles under **GCC, Clang (LLVM), and MSVC** with
strict standards-conformance flags, so non-portable / extension-dependent code
fails the build.

## Design

### Language inference & toolchain

- `inferLanguage` recognizes two new path components: `c` and `cpp`. The match is
  by *exact* path component, so `c` matches only a literal `code/**/c/**`
  directory — `csharp` and `cpp` directories are unaffected.
- A single CI toolchain named `cpp` covers **both** the `c` and `cpp` languages,
  because they share compilers (`gcc`/`g++`, `clang`/`clang++`, `cl.exe`). This
  mirrors the existing `csharp`/`fsharp` → `dotnet` collapse in
  `toolchainForPackageLanguage`.
- Dependencies of C/C++ packages are declared via the `# build-tool: deps=…`
  BUILD comment (already supported and used by `conduit`), **not** a package
  manifest. Therefore `c`/`cpp` are deliberately kept **out** of the validator's
  `requiresExplicitPrereqs` set so promotion from `"unknown"` to a real language
  does not start demanding `../`-style refs in the BUILD line.

### Compiler coverage = across the CI matrix

No single machine has all three compilers: MSVC exists only on Windows, and a
real GCC only on Linux (macOS's `gcc` is a Clang shim). Coverage is therefore
achieved **across** the 3-OS pull-request matrix:

| Runner         | Compilers exercised            |
| -------------- | ------------------------------ |
| ubuntu-latest  | GCC (`g++`/`gcc`) **and** Clang |
| windows-latest | MSVC (`cl.exe`) (+ optional clang-cl) |
| macos-latest   | Apple Clang                    |

Every pure-ISO C/C++ package is compiled by GCC, Clang, and MSVC across a single
PR CI run.

### Standards & strict flags

- **C17** and **C++17**.
- GCC / Clang: `-std=c17` / `-std=c++17 -pedantic-errors -Wall -Wextra -Werror`.
- MSVC: `/std:c17` / `/std:c++17 /permissive- /W4 /WX` (`/EHsc` for C++).

`-pedantic-errors` (GCC/Clang) and `/permissive-` (MSVC) are the switches that
actually reject non-ISO extensions; `-Werror` / `/WX` make every diagnostic fatal.

### Reusable ISO harness (later PR)

A shared, self-verifying harness at `code/packages/c/iso-harness/` compiles each
translation unit with **every** compiler present on the runner under the strict
flags, fails if zero suitable compilers are found, and supports an optional
`ISO_REQUIRE="gcc clang"` guard so CI can assert that both GCC and Clang actually
ran (making the guarantee firm, not best-effort). It also ships a header-only,
ISO-only test harness (`iso_test.h`) so sample packages need no external test
dependency.

### Existing packages

`cpp/conduit`, `cpp/conduit-hello`, and `cpp/mosaic-flux-qt` use POSIX sockets,
Qt, and Rust FFI; they cannot be pure ISO and keep their current
single-compiler builds. Promotion to language `cpp` does not change how they build
(the build tool runs every affected package regardless of language; language only
drives toolchain installation). Their BUILD lines contain no `../` refs, so the
validator's ref checks skip them exactly as before.

## Rollout (PRs)

1. **PR1 (this):** spec + `inferLanguage` recognizes `c`/`cpp`; `cpp` added to
   `allToolchains`; `c`/`cpp` → `cpp` in `toolchainForPackageLanguage`; tests.
2. **PR2:** ci.yml `needs_cpp` output + normalize + install (clang on ubuntu,
   MSVC on windows); `cpp` added to the validator's `ciManagedToolchainLanguages`.
3. **PR3:** shared `code/packages/c/iso-harness/` (self-testing).
4. **PR4:** scaffold-generator `c`/`cpp` templates.
5. **PR5:** sample pure-ISO C (`c/ringbuf`) and C++ (`cpp/static-vector`)
   packages that exercise the harness end-to-end across the matrix.
