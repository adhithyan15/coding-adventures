# TF00 — Portable Test Framework

## Overview

This spec designs a test framework written entirely in-house and ported
across the monorepo's languages, replacing `vitest`, `jest`, `pytest`,
`minitest` and `simplecov`.

Two motivations, and they point the same way. The practical one: 92% of the
repository's Dependabot alerts root at exactly two declared dependencies,
`vitest` and `@vitest/coverage-v8` (B06 has the full attribution). A package
like `document-html-sanitizer` declares those two and installs **94
packages** to run its tests. The educational one: this repository implements
lexers, parsers, bytecode compilers, virtual machines and gate-level CPUs
from first principles. A test framework is a smaller and better-structured
artifact than any of those, and it is the one piece of infrastructure every
package already touches.

The spec's central claim is that this is much smaller than it sounds, and
the reason is measurable rather than optimistic. See "Measured surface"
below: the repo's 7,245 test files use about twenty distinct assertions.

## A warning about "rich"

The instinct with a test framework is to match the surface of the thing
being replaced. Vitest exposes several hundred matchers, mocking modes,
snapshot formats, browser modes, workspace configs and a UI. Reproducing
that across 22 languages is not a project, it is a career.

The measured usage says the opposite is wanted. This spec therefore targets
a **narrow API specified deeply** — roughly twenty assertions whose
semantics are pinned exactly, in every language, by a shared conformance
corpus — rather than a wide API specified loosely. Narrow-and-exact is also
the version that teaches more: the interesting content of a test framework
is not its matcher count, it is deep equality, test isolation, failure
diffing and coverage instrumentation.

## Measured surface

Measured at `d2610543e6` by grepping every test file in the repo. These are
call-site counts, not estimates.

### TypeScript (1,264 test files)

| Matcher | Uses | | Matcher | Uses |
|---|---:|---|---|---:|
| `toBe` | 27,096 | | `toBeDefined` | 520 |
| `toEqual` | 9,118 | | `toBeUndefined` | 492 |
| `toContain` | 4,736 | | `toBeGreaterThanOrEqual` | 340 |
| `toThrow` | 3,847 | | `toBeInstanceOf` | 290 |
| `toHaveLength` | 2,448 | | `toBeTruthy` | 289 |
| `toBeGreaterThan` | 1,314 | | `toThrowError` | 170 |
| `toMatch` | 1,187 | | `toStrictEqual` | 138 |
| `toBeNull` | 942 | | `toBeLessThanOrEqual` | 134 |
| `toBeLessThan` | 888 | | `toHaveBeenCalledWith` | 116 |
| `toBeCloseTo` | 745 | | `toHaveProperty` | 107 |
| `toMatchObject` | 698 | | `toHaveBeenCalledTimes` | 86 |

Structural API: `it` 32,096, `describe` 7,691, `test` 3,080, `beforeEach`
304, `it.each` 280, `afterEach` 212, `beforeAll` 109, `afterAll` 25.

Test doubles: `vi.fn` 340, `vi.mock` **8**, `vi.mocked` 2, `vi.hoisted` 1.

Snapshot testing: **zero uses.** No `toMatchSnapshot`, no
`toMatchInlineSnapshot`. The single hairiest feature in a modern test
framework — serializer registries, inline source rewriting, obsolete-snapshot
detection, update modes — is entirely absent from this codebase and is out
of scope.

DOM matchers (`toBeInTheDocument` 224, `toHaveTextContent` 27) come from
`@testing-library/jest-dom` and are confined to the 29 packages with a DOM
dependency. They are deferred to TF-DOM, below.

### Python (1,620 test files)

Bare `assert` 66,971; `pytest.raises` 3,547; `pytest.approx` 881;
`pytest.mark.parametrize` 554; `pytest.fixture` 164; `pytest.skip` 17.

Python's surface is almost entirely the language's own `assert`. What pytest
supplies that `unittest` does not is assertion *rewriting* — the bytecode
transform that turns a failed bare `assert a == b` into a readable diff.
That, plus `raises`, `approx` and `parametrize`, is the whole dependency.

### Ruby (711 test files)

`assert_equal` 13,073; `assert` 2,094; `assert_includes` 1,341;
`assert_raises` 1,210; `assert_in_delta` 639; `refute` 638; `assert_nil`
402; `assert_instance_of` 323; `assert_match` 321; `assert_empty` 174;
`assert_predicate` 12.

Eleven assertions.

### Go (548 test files)

`t.Errorf` 8,928; `t.Fatalf` 6,027; `t.Error` 3,317; `t.Fatal` 2,747;
`t.Run` 778; `t.Helper` 281; `t.Parallel` 55; `t.Skip` 50; `t.Cleanup` 13.

**Go needs nothing.** `testing` is stdlib, `go test -cover` is built in, and
the repo already uses only stdlib APIs. Go is the reference lane: it shows
what the end state looks like.

### Remaining lanes

Rust 1,662 test files, Elixir 443, Haskell 431, C# 214, Swift 200, Java 152.

## The crucial scoping decision

**Do not write 22 test runners.** Most of these languages already ship one:

| Language | Runner | Source | What is missing |
|---|---|---|---|
| Go | `testing` | stdlib | nothing |
| Rust | `cargo test` | toolchain | coverage reporting |
| Elixir | ExUnit | distribution | coverage reporting (`:cover` is OTP) |
| Python | `unittest` | stdlib | assertion diffs, parametrize, coverage |
| Node/TS | `node:test` | stdlib (Node 22) | matcher vocabulary, DOM |
| Ruby | — | `minitest` is a *bundled*, removable gem | runner + assertions |
| Java/C#/Swift/Haskell | JUnit/xUnit/XCTest/HSpec | varies | audit separately |

So the project is **not** "write Vitest 22 times". It is:

1. One **portable assertion vocabulary**, specified once as a conformance
   corpus.
2. Thin **per-language adapters** binding that vocabulary onto whatever
   runner the language already ships.
3. A real **runner** only where the language has none — realistically Ruby,
   and TypeScript only if `node:test` proves insufficient.
4. A **coverage story** per language, which is the genuinely hard part.

This is both dramatically less work and a better design: tests keep running
under the tooling each language's ecosystem already understands (IDE
integration, `cargo test` output, `go test -run`), while the assertion
semantics become ours and identical everywhere.

## Anatomy: the six separable pieces

Worth naming explicitly, because the educational value is in the seams. A
test framework is usually presented as one artifact; it is six, and they
have genuinely different difficulty.

**1. Discovery and registry.** How `describe`/`it` collect tests before any
run. This is a tree built by side effect: calling `describe(name, fn)`
pushes a node, runs `fn` immediately (which registers children), then pops.
The tests themselves are closures captured but not invoked. Once you see
that registration and execution are two separate passes, most of the
framework's surprising behaviour — why a `console.log` in a `describe` body
prints before any test runs, why `beforeEach` defined after an `it` still
applies to it — stops being surprising. *Difficulty: low. Teaching value:
high.*

**2. Assertions.** The vocabulary, and the failure messages. Easy in the
common case, and hiding one genuinely hard algorithm (deep equality, below).
*Difficulty: medium.*

**3. Execution and isolation.** Running the tree: hook ordering, nesting,
async, timeouts, failure containment, and how much isolation to offer
(vitest gives a worker per file; `go test` gives a goroutine per test).
*Difficulty: medium, and the main source of cross-language divergence.*

**4. Reporting.** Turning results into output. Solved once, portably, by
emitting **TAP version 14** — a line protocol that predates all of these
frameworks, that `node:test` already speaks, and that every language can
produce with string concatenation. *Difficulty: low. Do this early; it is
what makes 22 lanes comparable.*

**5. Coverage.** The one piece that is unavoidably per-language and
unavoidably deep, because it requires either compiler support or runtime
instrumentation. *Difficulty: high.*

**6. Test doubles.** Spies, stubs, fakes. `vi.fn` (340 uses) is a recording
wrapper and is nearly trivial. `vi.mock` (8 uses) is module-graph
interception and is not — at 8 call sites, rewrite them instead.
*Difficulty: low if module mocking is excluded, high if not.*

## Deep equality is the real project

`toEqual` has 9,118 call sites, `assert_equal` 13,073. Structural equality
is the single most load-bearing algorithm in the framework, and it is where
"obvious" implementations are wrong. The corpus must pin, per language:

- **`NaN`**. `NaN !== NaN`, but `expect(NaN).toEqual(NaN)` passes. Vitest
  uses `Object.is` semantics for primitives; C-family `==` does not.
- **`-0` vs `+0`**. `Object.is(-0, 0)` is false; `-0 === 0` is true. `toBe`
  and `toEqual` disagree here, and the corpus must say which is which.
- **Cycles.** `a.self = a` must compare without recursing forever, which
  needs a visited-pair set, not a visited-node set.
- **Sparse arrays, `undefined` members.** Is `[1, , 3]` equal to
  `[1, undefined, 3]`? `toEqual` says yes, `toStrictEqual` says no — the
  entire distinction between those two matchers (138 uses) lives here.
- **Maps and Sets.** Order-insensitive, and key comparison is itself a deep
  comparison.
- **Prototypes and classes.** `toEqual` ignores class identity;
  `toStrictEqual` does not.
- **Floats.** `toBeCloseTo` (745) and `assert_in_delta` (639) need a pinned
  tolerance *and* a pinned rule for what happens at infinities.
- **Cross-language numerics.** This repo has TypeScript packages using
  `bigint` for values that are `int` in Python and `i64` in Rust. The corpus
  must state whether `1` and `1n` compare equal, and every lane must agree.

That last point is why the corpus approach matters more here than in a
single-language framework: these questions have no default answer across 22
languages, and leaving them implicit guarantees the lanes silently diverge.

## The bootstrap problem

**How do you test a test framework?** Not with itself — a bug that makes
assertions always pass would make its own suite green. Not with vitest —
that is the dependency being removed.

This repo has solved the analogous problem before: it bootstraps compilers.
The same answer applies.

1. **The corpus is data, not code.** `cases.json` describes assertion
   inputs and expected verdicts declaratively — no executable assertions.
2. **A bootstrap harness runs it.** Forty-odd lines per language of
   deliberately boring code: read JSON, call the assertion, compare the
   verdict to the expectation, `exit(1)` on mismatch. It uses no framework,
   has no abstractions, and is short enough to verify by reading.
3. **Negative cases are mandatory.** Every assertion needs corpus entries
   that must **fail**. A framework that never reports failure passes any
   all-positive corpus. This is the single most important property of the
   corpus and the easiest to forget.
4. **Only then** does the framework's own test suite use the framework, for
   the ergonomic layers (hook ordering, reporting) that the corpus does not
   reach.

## Portable architecture

Follow the pattern the repo already uses for cross-language conformance.
`code/specs/fixtures/der-tlv-v1/` establishes it: `cases.json` (a
language-neutral behavioural corpus), `consumers.json` (a registry of
per-language implementations, each recording its idiomatic `surface`
naming), `consumers.schema.json`, and a CI gate asserting every consumer
passes every case. That fixture has 54 cases, 17 error ids and 15
established lanes, so the mechanism is proven at exactly this scale.

Proposed layout:

```
code/specs/fixtures/test-framework-v1/
  cases.json             assertion corpus: inputs, verdicts, failure ids
  consumers.json         per-language lane registry + surface naming
  consumers.schema.json
  schema.json
  README.md
```

`cases.json` entries name a **semantic assertion id**, not a matcher name,
because the matcher spelling differs per language while the semantics must
not:

```json
{
  "id": "deep-equal/nan-is-equal-to-itself",
  "assertion": "deep_equal",
  "actual":   {"kind": "number", "value": "NaN"},
  "expected": {"kind": "number", "value": "NaN"},
  "verdict":  "pass"
},
{
  "id": "identical/negative-zero-differs-from-positive-zero",
  "assertion": "identical",
  "actual":   {"kind": "number", "value": "-0"},
  "expected": {"kind": "number", "value": "0"},
  "verdict":  "fail",
  "failure_id": "not-identical"
}
```

`consumers.json` records each lane's naming, exactly as der-tlv does:

```json
{
  "language": "ruby",
  "package_root": "code/packages/ruby/coding_adventures_test",
  "fixture_test": ".../test/portable_conformance_test.rb",
  "surface": {
    "identical": "assert_same", "deep_equal": "assert_equal",
    "raises": "assert_raises", "contains": "assert_includes"
  }
}
```

## The portable assertion set

Derived from measured usage, intersected across languages. Each row is one
semantic id; the columns show how existing code already spells it.

| Semantic id | TypeScript | Python | Ruby | Rust |
|---|---|---|---|---|
| `identical` | `toBe` | `is` | `assert_same` | `ptr::eq` / `==` |
| `deep_equal` | `toEqual` | `==` | `assert_equal` | `assert_eq!` |
| `strict_deep_equal` | `toStrictEqual` | — | — | — |
| `contains` | `toContain` | `in` | `assert_includes` | `.contains()` |
| `raises` | `toThrow` | `pytest.raises` | `assert_raises` | `should_panic` |
| `has_length` | `toHaveLength` | `len(x) ==` | `.size` | `.len()` |
| `greater_than` | `toBeGreaterThan` | `>` | `>` | `>` |
| `less_than` | `toBeLessThan` | `<` | `<` | `<` |
| `matches` | `toMatch` | `re.search` | `assert_match` | `regex` |
| `is_null` | `toBeNull` | `is None` | `assert_nil` | `is_none()` |
| `is_defined` | `toBeDefined` | — | — | `is_some()` |
| `close_to` | `toBeCloseTo` | `approx` | `assert_in_delta` | `(a-b).abs()<e` |
| `matches_subset` | `toMatchObject` | — | — | — |
| `instance_of` | `toBeInstanceOf` | `isinstance` | `assert_instance_of` | — |
| `truthy` | `toBeTruthy` | `assert` | `assert` | `assert!` |
| `has_property` | `toHaveProperty` | `hasattr` | `respond_to?` | — |
| `is_empty` | — | — | `assert_empty` | `is_empty()` |
| `called_with` | `toHaveBeenCalledWith` | — | — | — |
| `called_times` | `toHaveBeenCalledTimes` | — | — | — |

Nineteen semantic assertions plus their negations. The dashes are
deliberate and important: they mark where a concept does not exist natively
in a language, and each one is a decision the spec must make explicitly
rather than paper over. `is_defined` is meaningless in Python, where there
is no `undefined` distinct from `None`. `matches_subset` and
`strict_deep_equal` are JavaScript-shaped and may not deserve lanes
elsewhere. **A lane is allowed to declare an assertion unsupported**; what
it is not allowed to do is implement it with different semantics.

## Coverage

The hard part, and the part with no shared implementation. Each language
needs its own, and every one of these is toolchain-provided rather than a
package:

| Language | Mechanism | Ships with |
|---|---|---|
| Go | `go test -cover` | stdlib |
| Rust | `-C instrument-coverage` + `llvm-profdata`/`llvm-cov` | `llvm-tools` rustup component |
| Node/TS | `--experimental-test-coverage`, `--test-coverage-lines=N` | Node 22 |
| Ruby | `Coverage` module | **stdlib** — replaces simplecov outright |
| Python | `trace`, or `sys.monitoring` on 3.12+ | stdlib |
| Elixir | `:cover` | OTP |

Every lane's coverage exists without a package manager. What has to be
written is the *threshold gate and report format* — small, and worth
unifying so `>80%` means the same thing everywhere. Note Node's threshold
flags genuinely gate: `--test-coverage-functions=90` exits 1 on a 66%
result (verified).

Python is the one lane needing an upgrade first: `trace` drives
`sys.settrace` on every line and will hurt on 1,620 test files.
`sys.monitoring` (3.12+) is the fast path.

## Sequencing

**TF01 — Corpus and TypeScript lane.** Write `cases.json` with negative
cases, the bootstrap harness, and the TypeScript lane over `node:test`.
Convert the 453 vitest packages with no DOM dependency. **This alone
retires ~92% of the repository's Dependabot alerts** and is the only phase
with a security payoff; everything after is craft.

**TF02 — Reporting.** TAP v14 emitter, shared shape across lanes.

**TF03 — Ruby lane.** The only language needing a real runner written
(minitest is a bundled, removable gem). ~150 lines: a base class, reflection
over `test_*` methods, the assertion set, `Coverage`-based thresholds.

**TF04 — Python lane.** Assertion vocabulary over `unittest`,
`parametrize` → `subTest`, `raises`/`approx` equivalents.

**TF05 — Rust and Elixir lanes.** Assertions and coverage only; both
runners stay.

**TF06 — Go lane.** Conformance test proving the corpus passes against
plain `testing`. No implementation — Go is the control.

**TF07 — DOM.** The 29 DOM packages. Node has no DOM; a CDP client over
Node 22's global `WebSocket` replaces jsdom *and* Playwright with better
fidelity. Largest remaining item.

**TF08 — Remaining lanes.** Java, C#, Swift, Haskell, Dart, Lua, Perl,
OCaml, F#. Audit each for what its ecosystem already provides.

TF01 is independent of everything else and carries all the security value,
so it should not wait on the architecture being finished.

## What this buys, and what it does not

It removes the two dependencies behind 92% of the alerts, and the ~90
transitive packages each one drags into ~450 lockfiles. It gives the repo a
single assertion vocabulary across 22 languages, pinned by a corpus, which
is something none of the upstream frameworks can offer. And it puts the
most-touched piece of infrastructure in the repo under the same
build-it-yourself treatment as its compilers and CPUs.

It does not make testing better. Vitest is a good tool and a hand-rolled
replacement will be worse at watch mode, editor integration, failure diffs
and error messages for a long time. Budget specifically for **failure output
quality**: a framework that reports *that* something failed but not *why* is
worse than the dependency it replaced, and this is where hand-rolled
frameworks usually disappoint. The measured 9,118 `toEqual` call sites are
9,118 places where a good structural diff pays for itself.

It also moves the maintenance in-house. A bug in our deep-equality
implementation is ours to find, with no advisory feed and no upstream issue
tracker — and a bug in a test framework is worse than most, because its
failure mode is tests that pass when they should not. That risk is the
argument for the corpus, the negative cases and the bootstrap harness being
non-negotiable rather than nice-to-have.
