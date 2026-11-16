# Interleaved 2 of 5 (ITF) Barcode Encoder for Kotlin

Pure Kotlin, dependency-free implementation of the Interleaved 2 of 5 (ITF) 1D barcode symbology conforming to AIM-BC2 and ISO/IEC 16390 specifications.

## Features

- Validates even-length numeric-only payloads up to 4096 Unicode scalar values.
- Interleaves pairs of digits into 10 runs (5 bars, 5 spaces; 2 wide, 3 narrow elements each).
- Projections into 1D runs and paint scenes via `barcode-layout-1d` and `paint-instructions`.
- Strict UTF-16 surrogate validation and payload-blind error handling.

## Usage

```kotlin
import com.codingadventures.itf.Itf

// 1. Normalize and validate input
val normalized = Itf.normalizeItf("0123456789")

// 2. Encode into interleaved digit pairs
val pairs = Itf.encodeItf(normalized)

// 3. Expand into 1D runs (start pattern + pairs + stop pattern)
val runs = Itf.expandItfRuns(normalized)

// 4. Project into a 2D vector paint scene
val scene = Itf.drawItf(normalized)
```

## 5-Class Barcode / ITF Security Resilience Showcase

| Class | Vulnerability / Threat | Architectural Defense & Mitigation |
|---|---|---|
| **Class 1: Strict Precedence Enforcement** | Ambiguous error precedence or boundary confusion | Bounded validation pipeline: null/lone surrogates fail first with `IllegalArgumentException`, scalar count `> 4096` fails with `input-too-long`, odd/empty length fails with `invalid-length`, non-ASCII digit fails with `invalid-character`. |
| **Class 2: Non-ASCII Digit Rejection** | CWE-176 Unicode homoglyph smuggling / localized digit confusion | Rejects non-ASCII digits (e.g. Arabic-Indic, fullwidth U+FF11 `１`) with `invalid-character`. Never uses permissive `Character.isDigit`. |
| **Class 3: High Capacity & Boundary Limit** | CWE-400 Memory amplification / unconstrained input DoS | Enforces exact 4096 Unicode scalar ceiling (`MAX_SCALAR_LIMIT`). Accepts exactly 4096 digits; rejects 4097+ with `input-too-long` prior to pair chunking. |
| **Class 4: Structural & Layout Invariants** | Barcode decodability failure / framing desynchronization | Guarantees standard start (`1010`) and stop (`11101`) framing, 10 runs per digit pair, 10-module quiet zones, and bijective recovery across all 100 digit pairs. |
| **Class 5: Payload-Blind Security** | CWE-209 Sensitive data leakage via exceptions or scene metadata | Error messages and default scene labels are strictly static and payload-blind. Sensitive numeric payloads (PINs, card numbers) never leak into diagnostic strings. |
