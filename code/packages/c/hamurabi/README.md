# hamurabi (C / ISO C17)

Pure, deterministic simulation core for the seminal 1968 mainframe resource allocation game (*Hamurabi*), implemented in pure ISO C17.

## Architectural & Verification Guarantees

- **Standard Conformance:** Pure ISO C17 (`-std=c17` / `/std:c17`) with `-pedantic-errors` and `/permissive-` under GCC, Clang, and MSVC.
- **Zero Heap Allocation (`VER07`):** Zero dynamic memory allocation (`malloc`, `calloc`, `realloc`, `free`) in the simulation core. All states are value structures and report formatting writes directly into a caller-supplied buffer with strict capacity validation.
- **NULL Pointer Confinement (`CWE-476`):** Every public function guards against NULL pointer dereferences and returns `HAMURABI_ERROR_NULL_POINTER`.
- **Signed Overflow UB Elimination (`CWE-190`):** `acres_delta` safely negates `INT32_MIN` by widening to `int64_t`. PCG32 PRNG bit-rotations guard shift amounts via bitwise masking `rot & 31u` / `(-rot) & 31u`. All intermediate multiplications and sums widen to `uint64_t` with explicit overflow checks.
- **Buffer Safety (`CWE-125` / `CWE-787`):** Teletype formatting uses bounded `snprintf` and caller capacity checks.
- **Zero Ambient Authority:** Closed capability boundary declared in `required_capabilities.json` (`"capabilities": []`).

## Building & Testing

Run tests via the shared `iso-harness`:

```sh
# POSIX / Linux / macOS
sh tools/run.sh

# Windows (PowerShell with cl.exe / MSVC on PATH)
powershell -NoProfile -ExecutionPolicy Bypass -File tools\run.ps1
```
