# VER05q — OCaml Property-Based Testing Engine

**Parent:** [VER05](VER05-property-engine.md), [VER05e](VER05e-java-pbt-engine.md) (the reference algorithms and the budget tree of §4.1), [VER05h](VER05h-ruby-pbt-engine.md), [VER05i](VER05i-c-pbt-engine.md). Siblings: Haskell VER05n, Perl VER05p, Lua VER05o (open on their own branches). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-OCAML-PBT. Governing contracts: [VER00 §7.2, §7.3](VER00-verified-shipping-program.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md).

This specification defines the OCaml 5 implementation of the Level 2 property-based
testing engine. OCaml had no test-bootstrap before this change. The engine conforms to the
`pbt-v1` corpus (`code/specs/fixtures/pbt-v1/cases.json`, 25 cases) with exact outputs,
minimal witnesses **and shrink step counts**, implements the verdict-honesty rules of
VER05 §7.1 and contains the hierarchical shrink-budget tree of VER05e §4.1 from the first
version (it is mandatory, not a follow-up).

The algorithms are those of the Java, Ruby, C and Haskell lanes and are not restated. This
document lists what is the same (§1), what OCaml changes (§3 to §7), and the tests that pin
each hazard (§9).

---

## 1. Same as the other lanes (normative by reference)

- VER05e §2: SplitMix64; `bounded(n)` with the rejection threshold `t = (2^64 - n) mod n`;
  `bounded_big` (k-word rejection above 2^64); FNV-1a 64; **`bounded(1)` consumes one
  word**; `sub_seed(run_seed, id) = SplitMix64(run_seed xor fnv1a64(utf8(id))).next_u64()`.
- VER05e §3: the generator algebra and its **draw order**, the cross-lane contract.
- VER05e §4: integrated shrinking in the order target probe, integer walk (bisection
  `current - floor(diff/2)` when `|diff| > 1`, else the unit step `current - sign(diff)`),
  sequence shrinker (empty or prefix probe, bisection left then right, single deletion right
  to left, element-wise walk left to right without a target probe). **`steps` counts accepted
  moves only**; the target probe is one move; `steps < max_steps` guards every loop head.
- VER05e §4.1: the budget tree (restated for OCaml in §5).
- VER05 §7.1 and VER05e §5: runner, closed failure vocabulary, verdict honesty, witness
  re-verification, CI redaction. VER05e §6: strict conformance runner, CWE-400 caps,
  negative controls.

## 2. Package and modules

`code/packages/ocaml/test-bootstrap`, opam package `coding-adventures-test-bootstrap`,
library `coding_adventures_test_bootstrap` (wrapped: `Coding_adventures_test_bootstrap.Gen`
and so on), OCaml 5.2.1, dune 3.16. **Zero runtime dependencies beyond the OCaml standard
library** (no `unix`, no `threads`, no Zarith). Test and dev dependencies are exactly those of
`ocaml/ct-compare`: `alcotest`, `bisect_ppx`, `ocamlformat`.

| Module | Role |
|---|---|
| `Bigint` | small arbitrary-precision signed integers (§3) |
| `Prng` | SplitMix64 on `int64`, `bounded`, `bounded_big`, `fnv1a64`, `sub_seed` |
| `Error` | the engine's exceptions |
| `Val` | the value model for generated values (§6) |
| `Escape`, `Canon` | total, capped canonical rendering and kind-sensitive equality (§7) |
| `Safe` | exception classification (§4) |
| `Types` | limits, failure ids, `Options`, `Verdict`, `Config` (sabotage hooks) |
| `Budget` | the budget tree and the ambient carrier (§5) |
| `Shrink` | integer walk, sequence shrinker, `shrink_int`, `shrink_list`, `shrink_bytes`, `simulate_replay` |
| `Gen` | the generator algebra and corpus entry points |
| `Runner` | `check`, verdicts |
| `Strict_json`, `Tfv`, `Values` | strict JSON, TFV v1 validator, strict corpus value decoding (§7) |
| `Corpus` | the pbt-v1 conformance evaluator |

The library performs no file, clock, network or environment access, so
`required_capabilities.json` declares only `thread:spawn` (the tests call `Domain.spawn`; the library
spawns nothing). The CI flag is a parameter
(`Types.ci_of_env (Sys.getenv_opt "CI")` is the caller's call), never read inside. The tests
embed the corpus as an OCaml string (`test/corpus_data.ml`), exactly like `ocaml/ct-compare`,
and the `BUILD` file checks the module against `cases.json` byte for byte with `cmp`, so a
fixture edit that forgets to regenerate the module fails the build. `cases.tfv` is the TFV
rendering of the same document; the package reads only `cases.json`.

## 3. Integers

- **`int` is 63-bit and cannot hold a `u64`.** SplitMix64 and FNV-1a run on `Int64`
  (wrapping, signed), the right representation for a word: `Int64.add`/`mul` wrap modulo 2^64
  and `Int64.shift_right_logical` is the unsigned shift. Every unsigned comparison, quotient and
  remainder uses `Int64.unsigned_compare`, `unsigned_div` and `unsigned_rem`.
- `bounded(n)` takes `n` in `[1, 2^64]`. The threshold is `Int64.unsigned_rem (Int64.neg n) n`,
  exact because negation wraps modulo 2^64 (the C spelling). The same expression on a signed
  remainder is wrong for `n > 2^63`; pinned at `n = 1, 2, 2^63, 2^63 + 1, 2^64 - 1`; `n = 2^64`
  is one raw word.
- **Every integer a generator, a fixture or a shrinker can carry is a `Bigint.t`** (u8, u64
  and i128 included, as in the Java, Go and Haskell lanes). `Bigint` is a hand-written
  sign-magnitude integer in base 2^30 limbs (a product plus carries fits a 63-bit `int`) with
  exactly what the engine needs: parse and print decimal, compare, add, subtract, multiply,
  truncated and floored division, shifts, bit length, conversion to and from `int64`. Division by
  a one-limb divisor is short division; the general case is shift-and-subtract
  (O(bits x limbs)), adequate because the widest divisor is the 16,384-bit span cap. Zarith
  would be a third-party dependency, so it is not used. No `int` arithmetic in the engine can
  overflow silently: sizes, counts and lengths are validated against caps before conversion from
  a `Bigint`, and the work counters use the overflow-free test `cost > max - used`.
- **Fixture integers are capped at 1,000 digits before parsing**; counts, lengths and retries
  at 10,000. A seed outside `[0, 2^64 - 1]` is a problem, never reduced modulo 2^64.
- `Gen.integer lo hi` refuses a span wider than 16,384 bits (CWE-400 on the k-word rejection
  loop). `Gen.int lo hi` is the native-`int` convenience over the same draws.

## 4. Exceptions, effects and classification

A property may raise anything. Rules:

- **Classification is by exception constructor only**, with `Printexc.exn_slot_name` (and
  pattern matches for the engine's own exceptions). **`Printexc.to_string` is never called:** an
  exception can carry a payload (including a secret) and a registered printer runs user code.
  Messages carry the constructor name, escaped and clipped to 200 characters; the payload is never
  rendered.
- `Sys.Break` (Ctrl-C when `Sys.catch_break true`) is **never swallowed**: it propagates out of
  `check` after the scope has been closed (also when it arrives wrapped in `Fun.Finally_raised`, which
  `Safe.recoverable` unwraps). `Out_of_memory` is **also re-raised**: the process is
  out of memory and the engine cannot be trusted to build a verdict. `Stack_overflow` **is a
  `property-panic`**: it is deterministic misbehaviour of the property and the stack has unwound
  when the handler runs (matches the Java lane). Where the runtime aborts instead of raising,
  nothing can be done in-process.
- An **unhandled effect** raised by the property is the exception `Effect.Unhandled`; it is a
  panic like any other exception. The engine installs no effect handler and never captures a
  continuation, so a property that performs effects under its own handler is unaffected.
- Scopes are closed with `Fun.protect ~finally`, so they close however the body is left,
  including `Sys.Break`. The `finally` function cannot raise.
- Engine exceptions: `Argument_error` (a public entry point called with arguments it refuses;
  never a pass), `Not_falsifying` (a shrinker started from a value that does not falsify),
  `Exhausted` (generation could not produce a value). The runner treats `Exhausted` and any
  exception from the sampler as `property-exhausted`.

## 5. The shrink-budget tree

A direct port of VER05e §4.1, with the Haskell lane's decisions where Java's were weaker. A
scope is a node with a `parent`, a `depth`, a count of enclosing checks and caps, two counters
(**attempts**, one per property evaluation, cap `16 * max_steps + 64`; **work**, elements
copied, at least 1 per charge, cap `max_shrink_work` = 64M), a `closed` flag and a `refused`
flag **shared by the whole tree**.

- **All or nothing.** `charge b n` (an attempt) and `charge_copy b n` (work only, so attempt
  counts and the corpus are unchanged) walk from `b` to the root, require on every node `not
  closed`, `attempts < max_attempts` and `cost <= max_work - work` (the overflow-free test; an
  absurd cost is refused), then increment every node. A refusal sets `refused` and leaves every
  counter unchanged. The walk runs under **one mutex owned by the root and shared by every node
  of the tree**, and **no user code runs while it is held**, so concurrent charges from several
  domains are atomic. Callers charge **before** building a candidate; once refused, every loop
  ends.
- **Accounting.** One property evaluation is exactly one attempt and one unit of work, charged
  by the `falsifies` function the entry point hands to a shrinker; sequence shrinkers additionally
  charge the **copy** before building each candidate and never re-charge the evaluation.
- **Public entries open a child, internal combinators join.** `shrink_int`, `shrink_list`,
  `shrink_bytes`, `simulate_replay`, `Gen.shrink` and the runner's shrink open a **child** of the
  scope they are given with a fresh local cap and charge every evaluation including the initial
  one. `tuple`, `list`, `filter` and `bytes` shrinkers **join** the node they are handed. The
  chain always reaches the outermost root, so the total work of one top-level `check` is capped by
  that root whatever the nesting. Owning nesting is capped at depth 64; deeper scopes join.
- **`check` wraps its whole body, case loop included,** in one scope (a root with no parent, a
  child otherwise) with attempt cap `num_tests + 16 * max_steps + 64`. A check **nested** in a
  property pays one attempt at entry (refused: `property-exhausted`, nothing evaluated), one per
  case after the first, the work of every sampled value, and one attempt per filter retry. A
  starved nested check is exhausted, never a pass; a **top-level** check has no parent and always
  evaluates its first case. Top-level filter retries are charged as work (`charge_copy 1`) because
  the top-level attempt cap would wrongly exhaust a legitimate rare filter; `max_retries` is at
  most 10,000.
- **Nesting is capped.** A check whose parent already has 64 enclosing checks, or sits at depth
  63 or more, evaluates nothing, marks the tree refused and returns `property-exhausted`.
- **Starvation is never a pass.** If any charge in the tree was refused, a top-level check that
  would pass returns `property-exhausted`; a falsified verdict is never overridden. The single
  re-verification charges one attempt to the check scope; if refused, the shrunk value is not
  adopted and the **original witness is reported with 0 steps**.
- **A closed or stale handle is spent.** A node whose own `closed` flag, or any ancestor's, is
  set refuses every charge. There is no resolution to a nearest open ancestor and a closed handle
  is never a way to get a fresh root.
- **Cooperative trust model.** The budget bounds the work of code that goes through the engine's
  entry points. It cannot bound a property that loops or allocates without ever calling the
  engine (OCaml cannot interrupt a running domain without `Unix` signals, which are not a
  dependency), and it does not stop code that deliberately opens a fresh root and spends that. A
  caller that needs a wall-clock bound runs `check` under its own watchdog, which works because
  `Sys.Break` is never swallowed.
- Generation work is approximate: a sampled value's work is its collection length plus, one level
  deeper, the length of each child collection, byte string or text, visiting at most 10,000
  children, computed over the canonical `Val` view so a throwing `canon` costs 0.

### 5.1 The budget carrier (decision): ambient `Domain.DLS`, explicit override

| Design | Result |
|---|---|
| Global mutable variable | Rejected: shared across domains, races, leaks across tests |
| Explicit parameter threaded through every property (`Budget.t -> 'a -> bool`) | Rejected as the only mechanism: every simple property pays for the rare nesting case; kept as an **override** |
| **`Domain.DLS` with `~split_from_parent`, plus an optional `?parent` argument** | **Chosen** |

The ambient scope is a `Domain.DLS` key whose `split_from_parent` is the identity, so a
**domain spawned inside a scope starts with that node** and charges the same tree (the analogue of
Java's `InheritableThreadLocal`, and the reason `DLS` was chosen over a plain `ref`). Every
function that can open a scope takes `?parent:Budget.t`; absent, the ambient scope of the calling
domain is used; present, it is used even if closed (every charge is then refused).
`Budget.current ()` returns the ambient handle. Consequences, each pinned by a test:

- a domain that outlives the check that spawned it holds a **closed, spent** handle. `Budget.current ()`
  resolves a closed node with an open ancestor to that ancestor; if the **whole chain** is closed (only
  possible for an ambient inherited by such a domain) it returns the spent node itself, so every charge
  is refused and a plain `check` there is `property-exhausted`, **never a fresh root** (the Elixir
  lane's rule). The `finally` of a scope writes back `skip_closed saved` (the first open ancestor, else
  none), never a closed node, so restore-ordering staleness cannot leave a closed ambient and a later
  plain check after interleaved scopes is a fresh root and passes;
- a domain that was started before the scope existed has no ambient scope, so a top-level `check`
  it calls gets a **fresh root** bounded by that root only (the documented escape shared with the
  Haskell `forkIO`, Elixir process dictionary and Swift thread-local);
- the ambient value is restored in `Fun.protect ~finally` **only if the closing scope is still the
  current one**, so interleaved scopes cannot write a closed node back. The ambient scope is **per
  domain, not per thread or fibre**: concurrent checks in different systhreads or effect fibres of one
  domain must pass `?parent` explicitly (a regression test interleaves two fibre scopes). Fibres or
  threads running checks concurrently in one domain without `?parent` may be charged to another
  check's tree; if that tree closes first they get `property-exhausted`: fail-closed, never a pass. Suspending a fibre with an effect handler across a scope boundary
  is outside the contract (the ambient value is per domain, not per fibre);
- counters are shared between domains through the root's mutex, so charging concurrently never
  loses or doubles a charge and the tree never exceeds a cap.

### 5.2 Generation is bounded per level, not in total

As in the Haskell lane, `list` and `bytes` cap the length at each level, not the total: a list of
lists of up to 100,000 elements can produce 100,000 x 100,000 elements. Shrinking, by contrast,
shares one work budget across all levels. A caller that nests generators bounds the inner lengths.
`frequency` rejects every weight above 2^32 before summing, so the total cannot wrap a 63-bit
`int`.

## 6. Values

Generated values are arbitrary OCaml values, and the engine must render and compare them without
trusting them. Polymorphic `compare` and `(=)` raise on functional values and loop on cyclic data,
and `Obj` magic on user values is neither portable nor safe, so the engine **never applies
polymorphic compare, equality, hashing or `Obj` to a user value**. Instead:

- `Val.t = Nil | Bool of bool | Int of Bigint.t | Bytes of string | Text of string | List of
  t list | Tuple of t list` is the explicit canonical form (the Haskell lane's `Val`).
- **Every `'a Gen.t` carries a `canon : 'a -> Val.t`** (the Haskell `Canonical` class made an
  explicit field). Built-in generators supply it; `Gen.map ~canon` and `Gen.custom ~canon` take one
  from the caller. Rendering the witness and measuring work go through `canon`; a `canon` that
  raises turns the whole witness into `<unrenderable>` (never an engine crash).
- A property is `'a -> bool` and **holds iff it returns exactly `true`**. A `Val` can be cyclic
  (`let rec v = Val.List [v]`); rendering is depth-capped (16) and node-capped, and equality
  (`Canon.same_val`) is capped at 10,000 nodes and depth 64 and answers `false` past either cap.
- **Invalid generators fail closed.** A constructor given invalid arguments (`min > max`, an empty
  `one_of`, a zero-total `frequency`) returns a `Gen` whose sampler raises `Argument_error`; `check`
  reports `property-exhausted`, so an invalid generator can never pass. Public non-generator entry
  points raise `Argument_error`.
- Tuples are `tuple2` and `tuple3`, shrinking one coordinate at a time in passes until a pass makes
  no progress.

## 7. Canonical rendering, strict parsing, runner and conformance

`Canon.render : Val.t -> string` is total and bounded.

```text
Nil            nil            Bool          true / false
Int            decimal; above 100,000 bits  <bigint N bits>
Bytes          0x<lowercase hex>
Text           "quoted", with \" \\ \n \r \t and \uXXXX escapes below
List           [a,b]          Tuple         (a,b)
```

- **Caps, enforced as the value is traversed and before allocation:** nesting deeper than 16
  renders `...`; more than 10,000 nodes truncates; the output is capped at 1 MiB; a text or byte
  string is clipped to the remaining output budget **before** it is escaped or hex-encoded.
- **Escapes** (copied exactly from the Elixir/Haskell lanes, VER05m §3): every C0 control, DEL
  (U+007F) and C1 control (U+0080 to U+009F); U+2028 and U+2029; the bidirectional controls U+061C,
  U+200E, U+200F, U+202A to U+202E and U+2066 to U+2069; the invisible formatting characters U+00AD,
  U+034F, U+180E, U+200B to U+200D, U+2060 to U+2064, U+FEFF, U+FFF9 to U+FFFB and the tag block
  U+E0000 to U+E007F. `\n`, `\r`, `\t` have short forms; every other escaped character is `\uXXXX`
  (four lowercase hex digits), or `\u{XXXXX}` above U+FFFF. `"` and `\` are escaped only inside a
  quoted text. A byte that is not part of valid UTF-8 is written `\xHH` (a case the Haskell lane
  cannot have).
- `Strict_json.parse`: at most 8 MiB, at most 200,000 values (object keys count), container nesting
  at most 64 (**the 65th is refused before recursing, so native stack depth is bounded by the
  cap**); invalid UTF-8, a byte-order mark, raw control characters in strings, lone or reversed
  surrogate escapes, leading zeros, `+1`, `NaN`, comments, trailing commas and trailing data
  refused; duplicate object keys refused, compared by decoded bytes; numbers kept as text.
- `Tfv.validate` is VER05d's validator (header `tfv 1`, one space between tokens, lowercase even
  hex, canonical integers of at most 1,000 digits, counts that agree and are never used to
  allocate, no duplicate field, record id or map key, strict UTF-8 in `S`, depth 64, 1 MiB,
  200,000 tokens), so `corpus_reject` is checked by a real parser.

`Runner.check ?parent ?shrink_override ~seed gen options prop` runs `num_tests` (default 100)
cases from `SplitMix64(seed)` and always returns a verdict; invalid options raise
`Argument_error` (never a pass).

- `num_tests = 0` → **fail `property-exhausted`** (VER05 §7.1.1).
- a generator error of any kind (an exception from the sampler, an exhausted filter, a refused
  budget) → `property-exhausted`.
- a recoverable exception from the property → `property-panic` (§4).
- the property returns `false` → shrink, then `property-counterexample`; every case held → pass.
- the reported witness is **re-verified**: if the shrinker raises, returns a value that no longer
  falsifies, a value on which the property raises, or an out-of-range step count, the original
  candidate is reported with 0 shrink steps. While shrinking, a candidate on which the property
  raises counts as *not falsifying*.
- `property-timeout` is registered and never produced (VER05 §7.4 #5).
- **CI redaction**: `Ci_unset` is treated as CI (fail closed). A redacted verdict carries no seed,
  no counterexample and no minimal value; its message carries the iteration, the step count, the
  exception constructor name for a panic and `repro-fnv`, the FNV-1a of
  `iter:<i>;steps:<s>;shrunk:<canonical>` as 16 lowercase hex digits (an unkeyed, non-secret fingerprint
  for correlation: anyone who can guess a small witness can confirm the guess by brute force, so it is
  not a redaction guarantee; VER05 §7.4 #3).

`Corpus.evaluate` strictly decodes every case (exactly the fields of its op; ints canonical decimal
of at most 1,000 digits; bytes even-length lowercase hex; an unknown op, property or malformed
value is a *problem*, never a skip), evaluates it with the engine's own public functions and
returns problem lines (none means conform). CWE-400 caps: counts, retries, lengths and list sizes
above 10,000 are problems; `count x max_len <= 2^20`; a `gen_*` case whose `outputs` length is not
`count` is refused before generating; at most 10,000 cases. Coverage gates: every op and all four
failure ids present, an unregistered id rejected, case count equal to 25.

The conformance test loads the embedded corpus, asserts zero problems and exactly 25 evaluated, and
prints `pbt-v1: 25/25`. The unit suite carries negative controls: for every real case it applies a
corruption its op defines (flipped output, wrong minimal, steps + 1, wrong failure id, flipped
validity, a valid TFV in place of a reject text) and asserts the run fails and every problem names
that case's id. Engine sabotage through `Config` (a changed bisection, a changed unit step, a
skipped deletion phase, a shifted RNG) must make the corpus run report problems.

### 7.1 Pinned values the corpus cannot catch

Shared with the C, Ruby, Haskell, Perl and Lua lanes (independent reference
`code/scripts/pbt_conformance.py`).

- **The rejection draw.** SplitMix64's finalizer is a bijection, so state 0 yields word 0; the seed
  `7046029254386353131` (`0 - 0x9e3779b97f4a7c15 mod 2^64`) starts a stream whose first word is `0`
  and whose second is `16294208416658607535`. `bounded(3)` has threshold 1, so the `0` **must be
  rejected**: the answer is `16294208416658607535 mod 3 = 1`.
- **The `2^63 + 1` boundary.** `n = 2^63 + 1` has threshold `2^63 - 1`. Seed 3's first word
  `2092789425003139053` is below it, so `bounded` answers from the second word,
  `3694763184872335752`; seed 1's first word is above it and is accepted, `1227844342346046656`.
- **The blind spot is real.** A rejection threshold of 0 still passes all 25 corpus cases; only the
  pins above catch it. The suite asserts both facts, so nobody mistakes 25/25 for proof that
  `bounded` is unbiased.
- **`bounded(1)` consumes a word:** after `bounded 1` from seed 7 the next word is
  `309689372594955804`.
- **Step counts and evaluation order:** `shrink_int 54321 (<=100)` is `(101, 15)`; everything
  failing is `(0, 1)`; `max_steps = 0` returns the start with 0; `max_steps = 3` returns exactly 3
  steps; a negative bisection floors (from `-81` toward 0, `floor(-81/2) = -41`, so the candidate is
  `-40`, not `-41`); `shrink_list [0,0,0,0,5,0,0,0]` ("does not contain 5") is `([5], 3)` with
  evaluation order initial, `[]`, `[0,0,0,0]` (rejected), `[5,0,0,0]` (accepted), `[5,0]`, `[5]`,
  then the element probes `[3]`, `[4]`; `shrink_bytes 0102030405060708 (len < 5)` is
  `(0000000000, 16)`; `simulate_replay 42 (<=100)` is `(413, 101)`.

## 8. Build and inputs

`BUILD` mirrors `ocaml/ct-compare`: `opam install . --deps-only --with-test --with-dev-setup -y`; a
`cmp` of the embedded corpus module against `cases.json`; `dune build @fmt`; `dune runtest` with
`bisect_ppx` instrumentation; `bisect-ppx-report summary --per-file` with one `--expect src/<file>.ml` per instrumented source
(every `.ml` under `src/` except `val.ml`, a type-only module with no instrumentable code; the directory
form `--expect src/` does not match Windows `src\` paths, so files are listed explicitly; `--expect` checks that every source file appears in the report; it is not a percentage gate, so
the changelog quotes the measured coverage). `BUILD_windows` runs the same steps. The package is a
repository-input consumer of `code/specs/fixtures/pbt-v1/` in the Go build tool
(`repository_inputs.go`), so a corpus edit selects it and a missing package fails closed. A corpus
edit requires regenerating `test/corpus_data.ml` (the BUILD's `cmp` fails otherwise), exactly as
for CT01.

## 9. Tests

- Corpus replay (25 evaluated, zero problems), negative controls and sabotage; the pins of §7.1;
  the boundaries `n` = 1, 2, 2^63, 2^63+1, 2^64-1 against an independent bigint remainder; a
  threshold of 0 still passes the corpus.
- **Integers:** `Int64` wrap, unsigned compare/div/rem across the sign bit, `Bigint` round trips
  and division identities, a 1,001-digit fixture integer refused, a 1,000-digit one accepted, an
  out-of-range seed refused.
- **Exceptions:** `Failure`, `Not_found`, `Invalid_argument`, `Division_by_zero`, `Assert_failure`,
  `Stack_overflow`, a user exception with a payload and a registered printer (the printer must not
  run) and `Effect.Unhandled` are `property-panic`; `Sys.Break` and `Out_of_memory` propagate and
  the scope is closed afterwards; classification does not depend on message text.
- **Domains and effects:** a domain spawned in a scope charges the same tree (concurrent charging
  never exceeds a cap), a domain that outlives its scope is spent, a pre-existing domain gets a
  fresh root, an effect handled inside the property works.
- **User values:** a cyclic `Val`, a functional value inside a user type (never compared), a
  million-node and a 10 MiB value render within caps.
- **Budget tree:** the required set of VER05e's `ShrinkBudgetTest` (nesting depth 1 to 3 and
  branching, a recursing custom shrinker, a filter predicate that shrinks, a filter sampler
  rejecting everything, concurrent charging, a refused charge leaving counters unchanged, a huge
  cost, scope release after an exception, depth-cap joining, starvation never passing, a refused
  re-verification keeps the witness, top-level filter retries as work, a closed handle is spent).
- **Library hygiene:** strict JSON and TFV rejection lists, a 100,000-deep nesting refused without
  stack overflow, size and node caps, canon caps.
- **No hangs:** every runaway shape is bounded by the engine's own budget or caps; no test depends
  on a wall-clock alarm.

## 10. Divergences from other lanes

| # | Topic | OCaml | Others | Reason |
|---|---|---|---|---|
| 1 | property result | exactly `true` holds | Ruby truthy | fail-closed, typed `bool` |
| 2 | ambient scope | `Domain.DLS` (inherited by domains) plus `?parent` | thread-local / explicit monad | §5.1 |
| 3 | locking | one root mutex per tree | Java lock, Haskell STM | no STM in the stdlib |
| 4 | closed ancestor | spent: refuses every charge | Java charges it | stale handles are never a fresh root |
| 5 | exceptions | constructor name only; `Sys.Break` and `Out_of_memory` re-raised, `Stack_overflow` a panic | Haskell re-raises async | §4 |
| 6 | value model | explicit `canon` per generator | Haskell `Canonical` class | no typeclasses |
| 7 | integers | hand-written `Bigint`, `Int64` words | Haskell `Integer` | no Zarith dependency |
| 8 | user-exception text | never shown | Haskell shows `displayException` | `to_string` runs user printers |
| 9 | `property-timeout` | registered, never produced | same | VER05 §7.4 #5 |
