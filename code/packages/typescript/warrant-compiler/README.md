# @coding-adventures/warrant-compiler

Standalone, zero-runtime-dependency Warrant specification language compiler, AST parser, evaluator, and claim verifier.

## Overview

Part of the Verification Ladder (Level 1 & Level 2), `@coding-adventures/warrant-compiler` provides:
- **Bounded Lexer & Layout Capture**: Indentation-sensitive layout tokenizer conforming to VER04b and VER04c.
- **Warrant AST & Type Checker**: Comprehensive AST representation for models, operations, types, contracts (`requires`, `ensures`), claims, and concrete examples.
- **Pure Model Evaluator**: Deterministic evaluation of Warrant expressions and statements with loop fuel and call stack limits.
- **Universal Claim Verifier**: Automated verification of universal claims (`claim <name>: forall ...`) with counterexample extraction.
- **Zero Runtime Dependencies**: Completely self-contained (`"dependencies": {}`).

## Conformance
- [VER00 — Verification Manifesto](../../specs/VER00-verification-manifesto.md)
- [VER04 — Warrant Language](../../specs/VER04-warrant-language.md)
- [VER04a — Bounded Source Admission](../../specs/VER04a-warrant-source-admission.md)
- [VER04b — Exact Bounded Layout Capture](../../specs/VER04b-warrant-layout-capture.md)
- [VER04c — Guarded Warrant Parser](../../specs/VER04c-guarded-warrant-parser.md)
- [VER04e — Warrant Codegen & Test Vectors](../../specs/VER04e-warrant-codegen.md)
- [VER04f — Claims to Properties](../../specs/VER04f-claims-to-properties.md)
- [VER04g — TypeScript Warrant Compiler](../../specs/VER04g-typescript-warrant-compiler.md)

## Usage

```typescript
import { parseWarrant, executeWarrantModule } from "@coding-adventures/warrant-compiler";

const source = `
module math:
  op abs(x: i64) -> i64:
    requires: true
    ensures: result >= 0
    model:
      if x < 0:
        return -x
      return x
`;

const parsed = parseWarrant(source);
console.log(parsed.module.name); // "math"
```
