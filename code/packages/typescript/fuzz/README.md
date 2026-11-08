# @coding-adventures/fuzz

Standalone, zero-runtime-dependency TypeScript Level 2 coverage-guided fuzz testing engine with deterministic mutator algebra, 65,536-bit bitmap coverage tracking, and crash deduplication (VER06).

## Features

- **Zero Runtime Dependencies**: Strict `"dependencies": {}` guarantee.
- **Deterministic Mutator Algebra**: Bit flip, byte flip, byte arithmetic, boundary values, block deletion, insertion, duplication, and cross-seed splicing.
- **Coverage Feedback**: 65,536-bit (`COVERAGE_MAP_SIZE`) bitmap (`CoverageBitmap`) with hit detection and cumulative merging.
- **Corpus & Power Scheduling**: Dynamic energy allocation based on edge novelty and input length ($E \propto (1 + \text{new}/\text{total}) / \sqrt{L}$).
- **Crash Deduplication**: Normalized stack trace hashing via 64-bit FNV-1a crash signatures.
- **Crash Minimization**: Multi-phase input reduction (recursive chunk deletion with halving chunk sizes, followed by byte zeroing).
- **Closed Failure Vocabulary**: Strict classification (`fuzz-crash`, `fuzz-timeout`, `fuzz-oom`, `fuzz-leak`, `fuzz-panic`).
- **Privacy-Preserving TAP**: Emits anonymized TAP failure reports without leaking raw input payloads.

## Installation

```bash
npm install @coding-adventures/fuzz
```

## Quick Start

```typescript
import { fuzzTarget, CoverageBitmap } from "@coding-adventures/fuzz";

const report = fuzzTarget([new Uint8Array([0x01, 0x02])], { maxIterations: 500 }, (input, cov) => {
  if (input.length >= 4 && input[0] === 0xde && input[1] === 0xad) {
    cov.recordEdge(42);
    if (input[2] === 0xbe && input[3] === 0xef) {
      return { status: "crash", kind: "AssertionError", frames: ["main"] };
    }
  }
  return { status: "pass" };
});

console.log(`Executed ${report.iterations} iterations, found ${report.crashes.length} crashes.`);
```

## Specification Conformance

Conforms to VER06 and passes all 25 canonical test vectors in `code/specs/fixtures/fuzz-v1/cases.json`.
