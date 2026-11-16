# hamurabi (Java 21)

Pure, deterministic simulation core for the seminal 1968 mainframe resource allocation game (`GAME01`, `HA01`).

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis and programmed by William McKay for an IBM 7090 mainframe at BOCES in Westchester County, New York.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the DEC PDP-8 in FOCAL-69.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl.

## Architectural & Security Guarantees

- **Zero ambient authority (`VER00`):** `"capabilities": []` declared in `required_capabilities.json`.
- **Pure state transition (`VER04j`, `VER04m`):** `Hamurabi.stepYear(state, order)` produces a new immutable `HamurabiState` record without mutating inputs.
- **Explicit PRNG determinism:** 64-bit PCG32 XSH-RR PRNG state (`rngState`, `rngInc = 109`) embedded directly within `HamurabiState`.
- **Integer-only arithmetic (CWE-190 / CWE-681):** All calculations operate on 64-bit integers (`long`), using `Long.remainderUnsigned`, `Long.compareUnsigned`, `Long.divideUnsigned`, and `Integer.rotateRight` to avoid signed-overflow and precision hazards.
- **Negative bounds guards (CWE-190):** `acresDelta` explicitly guarded against `Integer.MIN_VALUE` before negation.
- **Zero serialization deserialization risks (CWE-502):** Pure Java 21 `record` types with no `Serializable` interfaces.

## Building and Testing

```sh
gradle test
```
