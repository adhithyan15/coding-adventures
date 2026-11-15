# hamurabi (C++ / ISO C++20)

Pure, deterministic simulation core for the seminal 1968 mainframe resource allocation game (*Hamurabi*), implemented in header-only pure ISO C++20.

## Architectural & Verification Guarantees

- **Standard Conformance:** Pure ISO C++20 (`-std=c++20` / `/std:c++20`) with `-pedantic-errors` and `/permissive-` under GCC, Clang, and MSVC.
- **Zero Raw Allocation (`VER07`):** Zero raw dynamic allocations (`new`, `delete`) in the simulation core. All state models and orders are `constexpr`-friendly value types with default comparisons.
- **Standard Bit Manipulation (`CWE-190`):** PRNG bit-rotations utilize standard `std::rotr` from `<bit>`.
- **Signed Overflow UB Elimination (`CWE-190`):** Safely handles `INT32_MIN` negation on `acres_delta` without undefined behavior by widening to `std::int64_t`. All arithmetic and validation bounds widen to 64-bit integer types with overflow guards.
- **Zero Ambient Authority:** Closed capability boundary declared in `required_capabilities.json` (`"capabilities": []`).

## Building & Testing

Run tests via the shared `iso-harness`:

```sh
# POSIX / Linux / macOS
sh tools/run.sh

# Windows (PowerShell with cl.exe / MSVC on PATH)
powershell -NoProfile -ExecutionPolicy Bypass -File tools\run.ps1
```
