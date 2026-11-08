# VER04g — TypeScript Warrant Compiler & Claim Verifier

**Parent:** [VER04 §5–§6](VER04-warrant-language.md), [VER04c](VER04c-guarded-warrant-parser.md), [VER04d](VER04d-warrant-type-system.md), [VER04e](VER04e-warrant-codegen.md), [VER04f](VER04f-claims-to-properties.md), [TF01a](TF01a-typescript-test-bootstrap.md), [VER05a](VER05a-typescript-pbt-engine.md).

This specification defines the TypeScript implementation of the Warrant v0 parser, evaluator, host binding compiler, and claim verifier within `@coding-adventures/test-bootstrap`.

It delivers **VER-P033**: compiling Warrant formal specifications (`.warrant`) into executable TypeScript unit and property-based tests running on `@coding-adventures/test-bootstrap`, verifying pure models, ground examples, input rejection contracts, and universal claims.

---

## 1. Architecture and Pipeline Placement

```text
┌────────────────────────────────────────────────────────┐
│ Warrant Source (.warrant text)                         │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────┐
│ Bounded Lexer & Layout Analyzer                        │
│ - Comments strip (# ...), Indent/Dedent capture        │
│ - Strict UTF-8, CRLF normalization, tokenization       │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────┐
│ Recursive-Descent Guarded Parser                       │
│ - Module, Errors, Models, Ops, Contracts, Bounds       │
│ - Examples (Ground Unit Tests)                         │
│ - Claims (Universal Quantified Properties)             │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────┐
│ Warrant Test Suite Compiler                            │
│ - Compiles examples to Level 1 unit tests              │
│ - Compiles claims to Level 2 Gen<T> property tests     │
│ - Dispatches bound operations against pure models      │
│ - Enforces error rejection and fresh output contracts  │
└────────────────────────────────────────────────────────┘
```

---

## 2. Language Subset & Expressions

The TypeScript Warrant compiler supports the pure v0 profile:

1. **Types**:
   - `Bool`, `Bytes`, `Bytes<N>`, `U8`, `U64`, `Nat`, `Int`, `Option<T>`, `Result<T, E>`.
2. **Expressions**:
   - Literals: `true`, `false`, `b"..."` (hex-encoded bytes), integers (`0u64`, `123`).
   - Equality & comparisons: `==`, `!=`, `<`, `<=`, `>`, `>=`.
   - Logical operators: `and`, `or`, `not`, `=>`.
   - Function & operation calls: `eq(a, b)`, `select_bytes(a, b, c)`.
   - Match expressions: `match e with None -> ... | Some(x) -> ...`.
   - Wrappers: `Ok(v)`, `Err(E.Variant)`, `Some(v)`, `None`.
3. **Declarations**:
   - `warrant <name>`: Module header.
   - `error <Name> = <Variant1> | <Variant2>`: Closed nominal error types.
   - `model <name>(<params>) -> <Type> = <expr>`: Pure reference model.
   - `op <name><generics>(<params>) -> <Type> [fresh]`: Public operation signature.
   - `input <op>.<param>`: Input contract (`hostile`, `rejects <pred> with <error>`, `ensures no_panic, terminates`).
   - `bound <op>`: Resource bounds (`steps <= ...`, `heap <= ...`, `depth <= ...`).
   - `example <name>`: Concrete input-output ground assertions.
   - `claim <name>`: Universally quantified property (`forall a: T, b: T [| P(a, b)]. condition`).

---

## 3. Claim-to-Property Compilation (VER04f)

Each `claim` compiles into a property test executed via `assertProperty`:
- Quantified binders `v: T` map to `Gen<T>` combinators:
  - `Bytes` $\to$ `Gen.bytes(0, 64)`.
  - `Bool` $\to$ `Gen.bool()`.
  - `U64` $\to$ `Gen.u64()`.
  - `U8` $\to$ `Gen.u8()`.
- Refinement conditions filter generated tuples with a retry budget (default 100).
- Bound host operations execute in sandbox isolation catching panics.

---

## 4. Ground Example Compilation (VER04e)

Each `example` compiles into a unit test running against the bound host API:
- Inputs are evaluated as ground values.
- Operation is invoked.
- Result is compared against expected outcome (value, `Ok`, or `Err`).
- If operation is marked `fresh`, output must not be reference-identical to inputs.

---

## 5. Conformance Verification

- Verified against canonical `code/specs/ct-compare.warrant`.
- All 11 examples and universal claims execute and pass against constant-time compare host implementations.
- Malformed inputs, missing bindings, or contract violations fail closed.
- Zero external runtime npm dependencies (`"dependencies": {}`).
