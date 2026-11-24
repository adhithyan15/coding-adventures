# VER05g — C# Property-Based Testing Engine

**Parent:** [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [VER05](VER05-property-engine.md), [VER05d](VER05d-go-pbt-engine.md), [VER05e](VER05e-java-pbt-engine.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-CS-PBT.

This specification defines the C# implementation of the Level 2 Property-Based
Testing (PBT) engine. C# had no test-bootstrap, no property engine and no fuzz
engine before this change. The engine conforms to the `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs, minimal
witnesses **and shrink step counts**, and implements the verdict-honesty rules of
VER05 §7.1.

It is a line-for-line port of the behaviour of [VER05e](VER05e-java-pbt-engine.md)
(Java), which has already been through three security-review rounds, including
every hardening item that review produced. This document therefore does not
repeat the algorithms; it states what is the same (§1), what C# changes (§2–§4)
and the tests that pin it (§5).

---

## 1. Same as Java (normative by reference)

The following VER05e sections apply to the C# engine unchanged, with C# names
substituted for Java ones:

- §2 PRNG: SplitMix64, `bounded`, `boundedBig`, `fnv1a64`, `subSeed`
  (`SplitMix64(seed ^ fnv1a64(utf8(id))).NextU64()`); `bounded(1)` consumes one word.
- §3 generator algebra and draw order (the draw order is the cross-lane contract).
- §4 integrated shrinking and the **global attempt budget**
  `16 * maxSteps + 64`, which counts rejected attempts and is shared by every
  pass and every tuple component (no fresh per-pass budget).
- §5 runner, closed failure vocabulary, verdict honesty, re-verification of the
  witness, CI redaction (`CiMode.Unset` is treated as CI).
- §6 conformance runner, strict decoding, CWE-400 caps, coverage gates, negative controls.

## 2. Package and namespaces

`code/packages/csharp/test-bootstrap` (.NET 9, `CodingAdventures.TestBootstrap.CSharp`).
One project, four namespaces:

| Namespace (`CodingAdventures.TestBootstrap.…`) | Role |
|---|---|
| `Prng` | `SplitMix64` (`ulong` state), `Bounded`, `BoundedBig`, `Fnv1a64`, `SubSeed` |
| `Pbt` | `Gen<T>`, `Gens`, `Shrink`, `Runner`, `Verdict`, `Options`, `CiMode`, `FailureIds`, `Limits`, `Corpus` |
| `CorpusJson` | `StrictJson`, `Values`, `Canon`, `Clip` |
| `Tfv` | `TfvValidator` (the class cannot be called `Tfv`: a type may not share its namespace's name) |

BCL only (`System.Numerics.BigInteger`). No third-party runtime dependency; xunit and
coverlet are test-time only. The library performs no file, clock, network or
environment access, so `required_capabilities.json` is capability-free. Tests read
the committed fixture by walking up to the repository root, never a copy.

## 3. C# specifics

- **Unsigned arithmetic is native.** SplitMix64 and FNV-1a use `ulong` (wrapping,
  `unchecked`). The rejection threshold is `(0UL - n) % n` computed unsigned
  (`2^64 - n` modulo 2^64); a signed or `Python (-n) % n` form would be 0 and never
  reject. `Bounded(0)` throws `ArgumentOutOfRangeException`.
- **Integer widths.** `U8()` yields `byte`, `U64()` yields `ulong` (C# has the
  unsigned types Java lacks, so VER05e §9 row 5 does not apply), `Int64(min,max)`
  yields `long`, `I128()` and `Integer(min,max)` yield `BigInteger`. All shrink
  through the same `BigInteger` walk. `BigInteger >> 1` floors on negatives, as
  required.
- **Conventions.** Properties are `Func<T,bool>` (`true` = holds). Invalid
  arguments throw `ArgumentException` subclasses; a start value that does not
  falsify throws `NotFalsifyingException`. Tuples are `Pair<A,B>` and `Triple<A,B,C>`
  (records that also implement `ITuple` so `Canon` renders them as `{a,b}`).
- **Recoverable failure.** The runner, `Canon` and the corpus evaluator catch
  `Exception` **except** `OutOfMemoryException`. `StackOverflowException` cannot be
  caught in .NET, so depth is capped explicitly: `Canon` renders nesting deeper than
  16 as `...`, `StrictJson`, `Values` and the TFV validator cap at 64, and
  `Gen.Sample` / `Gen.Shrink` call `RuntimeHelpers.EnsureSufficientExecutionStack()`
  so runaway generator nesting becomes a catchable `InsufficientExecutionStackException`
  (reported as `property-exhausted`).
- **Rendering.** `Canon` quotes strings with `\"`, `\\`, `\n`, `\r`, `\t` and
  `\u00xx`, renders numbers with the invariant culture, hashes
  `iter:<i>;steps:<s>;shrunk:<canonical>` with FNV-1a for `repro-fnv`, and degrades
  an unrenderable witness (throwing `ToString`, enumerator, dictionary) to the
  constant `<unrenderable>`. A `BigInteger` above 100,000 bits renders as
  `<bigint N bits>` (decimal conversion of a huge value is itself a DoS).
- **JSON.** Hand-written strict parser (`StrictJson`): depth 64, 8 MiB, 200,000
  nodes, duplicate keys refused, strict UTF-8 (`UTF8Encoding(throwOnInvalid: true)`,
  no BOM tolerance), no leading zeros, no trailing data, numbers kept as literal text.
- **Hardening carried from Java review:** every public count/size entry point is
  capped (`Limits.MaxGenCount` = 1,000,000), `GenBytes` is capped even when
  `maxLen == 0`, `GenBounded` is bounded by `count × byte width ≤ 2^20`, sequence
  and integer shrinking run under one global budget, the deletion phase copies each
  candidate once, CI-redacted verdicts carry no seed/witness/minimal.
- **Sabotage hooks.** `internal static class Hooks` (bisection, unit step, deletion
  phase, RNG factory) is visible to the test assembly through `InternalsVisibleTo`;
  tests reset it in a `finally`/`Dispose`. Test parallelism is disabled for the
  assembly because the hooks are process-global.

## 4. Divergences from Java and Go

| # | Topic | C# | Java / Go | Reason |
|---|---|---|---|---|
| 1 | `u64`/`u8` | `ulong`/`byte` | Java `BigInteger`/`Integer` | C# has unsigned types |
| 2 | stack overflow | cannot be caught; explicit depth caps and `EnsureSufficientExecutionStack` | Java catches `StackOverflowError` | .NET terminates the process |
| 3 | OOM | `OutOfMemoryException` never caught | same idea (`OutOfMemoryError`) | VM cannot continue safely |
| 4 | tuples | `Pair`, `Triple` implement `ITuple` | Java records via reflection | cheaper, no reflection |
| 5 | huge `BigInteger` render | `<bigint N bits>` above 100,000 bits | not capped in Java | .NET decimal conversion is super-linear |
| 6 | TFV class name | `TfvValidator` | `Tfv` | namespace/type clash |
| 7 | seed type | `ulong`; corpus seeds reduced mod 2^64 with `& ulong.MaxValue` | `long` | native unsigned |
| 8 | `Verdict.Seed` | `ulong?` | `Long` | as above |

## 5. Build and inputs

`BUILD` and `BUILD_windows` run `dotnet test … /p:CollectCoverage=true
/p:Threshold=95 /p:ThresholdType=line`. The package reads
`code/specs/fixtures/pbt-v1/` in its tests, so `csharp/test-bootstrap` is registered
as a repository-input consumer of that directory in the Go build tool
(`repository_inputs.go`), alongside python, go, java and kotlin.

## 6. Tests

- Replay of the committed corpus: exactly 25 evaluated, zero problems.
- Negative controls: for every case and every corruption its op defines, the run
  fails and every problem starts with that case's id and no other; dropped and
  truncated corpora fail; the engine hooks make the corpus run fail.
- Pinned tests for what the corpus cannot catch: a seed whose first draw lies below
  the rejection threshold (the SplitMix64 finalizer is invertible; output 0 needs
  state 0, i.e. seed `0 - 0x9e3779b97f4a7c15`), the `2^63+1` threshold boundary, and
  bisection accepting the left half then the right half with exact step counts.
- A regression test per hardening item of §3.
- At least 95% line coverage, enforced by the BUILD.
