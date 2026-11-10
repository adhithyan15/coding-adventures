# TF01a — TypeScript Bootstrap Harness (L0) & Zero-Dependency Test Runner

**Status:** Implementation in progress. Spec-first delivery under TF00, TF01, and VER00 §7.1.

**Layer:** Repository-wide test framework and zero-dependency tooling.

**Builds on:**
- [TF00 — Portable Test Framework](TF00-portable-test-framework.md)
- [TF01 — The Bootstrap Harness (L0), TFV, and the Run Protocol](TF01-bootstrap-harness.md)
- [VER00 — Verified Shipping Program](VER00-verified-shipping-program.md) (§2.1, §5, §7.1, §10 Phase 0)
- `code/specs/fixtures/test-bootstrap-v1/` (`cases.json`, `cases.tfv`)

---

## 1. Motivation and Boundary

Over 90% of repository security vulnerabilities and Dependabot alerts originate from third-party JavaScript/TypeScript testing dependencies (`vitest`, `@vitest/coverage-v8`, and their transitive tree of over 90 packages per package).

This specification establishes the official **TypeScript lane** of the Level 0 bootstrap harness (`test-bootstrap`), providing:
1. **Zero External Runtime Dependencies:** The package relies purely on the ECMAScript standard library and Node.js built-ins (`node:test`, `node:assert`, `node:fs`, `node:path`, `node:process`). `package.json` contains an empty `"dependencies": {}`.
2. **Canonical TF01 Conformance:** Directly executes and validates the full `test-bootstrap-v1` self-corpus (all 1,256 cases in `cases.json` and `cases.tfv`).
3. **Pure Deep Equality:** Bit-exact IEEE-754 float comparison (preserving `-0 !== +0` and canonicalizing NaN payloads), arbitrary-precision integers up to 1000 digits via native `BigInt`, byte sequences (`Uint8Array`), strings, lists, and maps.
4. **Deterministic Run Protocol:** Seeded pseudo-random execution with SplitMix64, FNV-1a 64 test-id hashing, Fisher-Yates array shuffling, isolated per-test sub-seeds, panic/exception containment, and TAP version 14 output.
5. **Ergonomic Level 1 Matchers:** Drop-in assertions (`describe`, `it`, `test`, `expect`, `beforeEach`, `afterEach`, `beforeAll`, `afterAll`) allowing pure TypeScript packages to drop `vitest` and `jest` completely.
6. **Built-in Coverage Gating:** Native coverage measurement using Node 22's `--experimental-test-coverage`.

---

## 2. Package Architecture

Package root: `code/packages/typescript/test-bootstrap`
Package name: `@coding-adventures/test-bootstrap`

```
code/packages/typescript/test-bootstrap/
  ├── package.json               # Zero runtime dependencies; devDeps: typescript only
  ├── tsconfig.json              # Strict ES2022 module configuration
  ├── BUILD                      # Standard build & test script
  ├── BUILD_windows              # Windows build & test script
  ├── CHANGELOG.md
  ├── README.md
  ├── required_capabilities.json # Audited capabilities (fs:read, proc:exit)
  ├── src/
  │   ├── types.ts               # Core AST, token, and verdict types
  │   ├── tfv.ts                 # TFV v1 zero-dependency parser & validator
  │   ├── verdict.ts             # Closed verdict vocabulary (Pass, Fail, failure IDs)
  │   ├── deep_equal.ts          # Pure deep equality matching TF01 §3.1
  │   ├── prng.ts                # SplitMix64, FNV-1a 64, bounded draw
  │   ├── harness.ts             # L0 runner, shuffling, isolation, TAP v14 reporter
  │   ├── matchers.ts            # expect() assertions built on deep_equal
  │   ├── runner.ts              # node:test bridge & CLI entry point
  │   └── index.ts               # Public exports
  └── tests/
      ├── conformance.test.ts    # 1,256 TF01 self-corpus test cases
      ├── sabotage.test.ts       # Mandatory negative failure detection
      └── matchers.test.ts       # Level 1 matcher unit tests
```

---

## 3. TFV v1 Parser Implementation Rules

The parser reads `cases.tfv` and enforces all TF01 §2 invariants:
1. **Header:** Exactly `tfv 1\n`.
2. **Comment/Blank Lines:** `#` comments and empty lines are skipped.
3. **Record Structure:** `<id> R<n> K<field> <value> ...`
4. **Value Prefixes:**
   - `N`: `null`
   - `T`: `true`, `F`: `false`
   - `I<int>`: Decimal integer up to 1000 digits (parsed as `BigInt`). Rejects leading zeros, `I-0`, non-digit characters.
   - `D<16 hex>`: 64-bit IEEE 754 float bit pattern (parsed using `DataView` and `BigInt`). Must be exactly 16 lowercase hex characters.
   - `X<hex>`: Byte string (`Uint8Array`). Must be lowercase hex of even length.
   - `S<hex>`: UTF-8 string encoded as lowercase hex.
   - `L<n>`: List of `n` values.
   - `M<n>`: Map of `n` key-value pairs.
5. **Nesting Depth Guard:** Maximum depth is 64. Depth 65 triggers immediate rejection.
6. **Strict Syntax:** Rejects duplicate fields, duplicate map keys, uppercase hex, trailing tokens, or multiple consecutive spaces.

---

## 4. Deep Equality Specification

`deepEqual(actual: Value, expected: Value): Verdict`

- **null / boolean:** Strict value identity.
- **integer (`BigInt`):** Mathematical integer equality.
- **float (`Float64`):**
  - If both are NaN: Equal (all NaN payloads compare equal).
  - If `-0` and `+0`: Not equal (`Object.is(-0, +0)` is false).
  - Otherwise: Exact numeric bit equality.
- **bytes (`Uint8Array`):** Same length and identical byte values at each index. Never equals text.
- **text (`string`):** Exact Unicode scalar sequence equality. Never equals bytes.
- **list (`Value[]`):** Same length and pairwise `deepEqual`.
- **map (`Map<Value, Value>`):** Same size, and for every key-value pair, there exists a deep-equal key with a deep-equal value.
- **different kinds:** Always returns `Fail("not-deep-equal")`.

---

## 5. Matchers API (Level 1)

`expect(actual)` provides:
- `.toBe(expected)`: Reference / primitive identity (`Object.is`).
- `.toEqual(expected)`: Pure deep equality via `deepEqual`.
- `.toStrictEqual(expected)`: Structural deep equality including prototype / property checks.
- `.toBeTruthy()`, `.toBeFalsy()`: Truthiness checks.
- `.toBeNull()`, `.toBeUndefined()`, `.toBeDefined()`: Null/undefined predicates.
- `.toBeGreaterThan(n)`, `.toBeGreaterThanOrEqual(n)`: Numeric order.
- `.toBeLessThan(n)`, `.toBeLessThanOrEqual(n)`: Numeric order.
- `.toBeCloseTo(n, precision = 2)`: Floating-point delta comparison.
- `.toContain(item)`: Substring or array inclusion.
- `.toHaveLength(len)`: Array/string length check.
- `.toThrow(expectedError?)`: Exception catching and message/error check.
- `.not`: Inverted matchers for all of the above.

---

## 6. Sabotage Mode & Failure Proof

Per TF01 §1.3 and §6:
Every harness must prove it detects failures. Running with `--sabotage` intentionally inverts one or more assertion outcomes or injects deliberate errors, verifying that:
1. The harness reports failure in TAP v14 (`not ok ...`).
2. The exit code is strictly non-zero (1).
3. The failure ID matches the closed failure vocabulary.

---

## 7. Migration Pilot: `code/packages/typescript/bitset`

As the first proof of Vitest retirement:
1. `code/packages/typescript/bitset` replaces `vitest` with `@coding-adventures/test-bootstrap`.
2. `package.json` eliminates `vitest` and `@vitest/coverage-v8` devDependencies.
3. `BUILD` and `BUILD_windows` use Node 22 native test runner:
   ```sh
   node --test --experimental-test-coverage
   ```
4. All existing tests pass with zero third-party packages installed.
