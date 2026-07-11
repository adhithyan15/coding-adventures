# iso-harness

A tiny, dependency-free **pure-ISO C/C++ build harness**. It compiles your
sources with **every C/C++ compiler present on the machine** — GCC, Clang
(LLVM), and MSVC — under the strictest standards-conformance flags, so any
reliance on a compiler-specific extension becomes a hard build error.

It is the shared engine behind the repo's C and C++ lane: new pure-ISO C/C++
packages call it from their `BUILD` / `BUILD_windows` scripts instead of
hand-rolling compiler selection.

## Why

Code that compiles on one compiler can still be non-portable — it may use a GNU
or MSVC extension the ISO standard does not guarantee. The only dependable check
is to compile with several independent compilers, each told to reject everything
the standard does not require:

| Compiler    | Standard flag         | Reject-extensions | Warnings-as-errors |
| ----------- | --------------------- | ----------------- | ------------------ |
| GCC / Clang | `-std=c17`/`-std=c++17` | `-pedantic-errors` | `-Wall -Wextra -Werror` |
| MSVC        | `/std:c17`/`/std:c++17` | `/permissive-`     | `/W4 /WX` (`/EHsc` for C++) |

`-pedantic-errors` (GCC/Clang) and `/permissive-` (MSVC) are the switches that
actually turn "uses a non-ISO extension" into an error.

## Coverage is across the CI matrix

No single machine hosts all three compilers — a real GCC lives only on Linux and
a real MSVC only on Windows (macOS's `gcc` is a Clang shim). The repo's CI runs
this harness on all three OSes, each contributing what it can host:

```
ubuntu  → gcc + clang       windows → cl (+ clang-cl)       macOS → Apple clang
```

The harness compiles with **whatever compilers it finds** and fails if it finds
none. To make coverage a hard guarantee rather than best-effort, set
`ISO_REQUIRE` (see below) so a missing expected compiler fails the build.

## Usage

From a consuming package's `tools/run.sh` (Unix):

```sh
# Point the harness at this package's headers and load the library.
HARNESS=$(cd ../../c/iso-harness && pwd)   # adjust the relative hops per location
ISO_INCLUDE="include"
export ISO_INCLUDE
. "$HARNESS/lib/iso-lib.sh"

# On ubuntu CI both gcc and clang are installed — require both so the guarantee
# is firm. (Locally, drop ISO_REQUIRE or set it to what you have.)
ISO_REQUIRE="gcc clang"
export ISO_REQUIRE

iso_build_and_run c   ring-tests   tests/ring_test.c src/ring.c
iso_build_and_run cpp vector-tests tests/vec_test.cpp
```

And from `tools/run.ps1` (Windows), via `lib/iso-lib.ps1`:

```powershell
. (Join-Path $harness 'lib\iso-lib.ps1')
$env:ISO_INCLUDE = 'include'
Iso-BuildAndRun -Lang c -Name ring-tests -Sources @('tests\ring_test.c', 'src\ring.c')
```

### Public interface

| Unix (`iso-lib.sh`)                         | Windows (`iso-lib.ps1`)                    | Purpose |
| ------------------------------------------- | ------------------------------------------ | ------- |
| `iso_compilers <c\|cpp>`                    | `Iso-Compilers -Lang c\|cpp`               | list present compilers |
| `iso_build_and_run <c\|cpp> <name> <src…>`  | `Iso-BuildAndRun -Lang … -Name … -Sources …` | compile with each compiler + run |
| `iso_expect_compile_fail <c\|cpp> <src>`    | `Iso-ExpectCompileFail -Lang … -Source …`  | assert non-ISO code is rejected |

### Environment knobs

| Variable        | Meaning                                                        | Default |
| --------------- | ------------------------------------------------------------- | ------- |
| `ISO_REQUIRE`   | space-separated compilers that MUST be present (else fail)     | *(none)* |
| `ISO_INCLUDE`   | space-separated include dirs (each becomes `-I`/`/I`)          | *(none)* |
| `ISO_CSTD`      | C standard                                                     | `c17`   |
| `ISO_CXXSTD`    | C++ standard                                                   | `c++17` |
| `ISO_BUILD_DIR` | output dir (never `build` — collides with `BUILD`)            | `_build` |

## Header-only test harness

`include/iso_test.h` is a single header — the intersection of ISO C17 and C++17,
so it compiles unchanged as either language — with no external test dependency:

| Macro | Checks |
| --- | --- |
| `ISO_CHECK(cond)` / `ISO_CHECK_MSG(cond, msg)` | a boolean condition |
| `ISO_CHECK_EQ_INT(a, b)` / `ISO_CHECK_EQ_UINT(a, b)` | signed / unsigned integer equality |
| `ISO_CHECK_STR_EQ(a, b)` | NUL-terminated C-string equality (accepts `std::string::c_str()`) |
| `ISO_CHECK_MEM_EQ(a, b, n)` | byte-wise equality of two buffers (hashes, cipher output) |
| `ISO_CHECK_EQ_DBL(a, b, eps)` | floating-point equality within a tolerance |
| `ISO_TEST_RESULT()` | prints a summary; returns 0 (all passed) or 1 |

Each failed check prints file, line, and the offending values, then keeps going
so one run reports every failure.

## Self-test

`sh tools/selftest.sh` (Unix) / `tools\selftest.ps1` (Windows) — run by
`BUILD` / `BUILD_windows`. It compiles a conforming C and C++ fixture (must build
**and run** on every compiler) and two non-conforming fixtures that use a GNU
statement expression (must be **rejected** by every compiler). The negative case
is the harness proving it actually enforces ISO conformance.

## Where it fits

Part of the C/C++ multi-compiler lane — see
[`code/specs/CCPP01-c-cpp-iso-multicompiler-lane.md`](../../../specs/CCPP01-c-cpp-iso-multicompiler-lane.md).
