# hamurabi (Kotlin/JVM)

Pure, deterministic simulation core for the seminal 1968 mainframe resource allocation game (`GAME01`, `HA01`).

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis and programmed by William McKay for an IBM 7090 mainframe at BOCES in Westchester County, New York.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the DEC PDP-8 in FOCAL-69.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl.

## Architectural & Security Guarantees

- **Zero ambient authority (`VER00`):** `"capabilities": []` declared in `required_capabilities.json`.
- **Pure immutable data models (`VER04m`):** `HamurabiState` and `TurnOrder` are immutable Kotlin data classes with val-only properties.
- **Explicit PRNG determinism:** 64-bit PCG32 XSH-RR PRNG state (`rngState`, `rngInc = 109`) embedded directly within `HamurabiState`.
- **Integer-only arithmetic (CWE-190 / CWE-681):** Bounded calculations using explicit `[0, 0xFFFFFFFFL]` bounds and safe integer arithmetic.
- **Negative bounds guards (CWE-190):** `acresDelta` guarded before negation to prevent signed 32-bit overflow on `Int.MIN_VALUE`.

## Building and Testing

```sh
gradle test
```
