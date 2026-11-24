# LG02 — Basic Gates: the cross-lane contract

**Status:** design, 2026-11-24. Records the coordinator decisions of the
"logic gates everywhere" program (backlog shard
`VER-BACKLOG.d/logic-gates-everywhere.md`, items LGE-S1a and LGE-S2). Nothing
in this document is implemented yet: it specifies a Warrant document, a
fixture, a drift script and per-lane error identities that later PRs build.
Every statement about existing code cites the file and line it was read from.

## 1. What this document is for

[LG01](LG01-logic-gates-contract.md) is the contract of the Rust package
`code/packages/rust/logic-gates`. It covers 22 operations, and its executable
form [`logic-gates.warrant`](logic-gates.warrant) is compiled into a Rust test
harness. Fifteen other language lanes have their own `logic-gates` package
(every one of the 18 lanes except C and C++, and Rust itself), and LG01 says honestly that "no other port has
been checked against this contract" (LG01, "Cross-language expectations").

This document picks the part of LG01 that every language can and must agree on,
the **eleven basic gates**, and fixes how that agreement is written down,
guarded against drift and (eventually) replayed in each lane. It also fixes how
each lane *reports* an invalid input, because the evidence tooling is not
allowed to read error messages (section 7).

A beginner's summary of the problem: a logic gate is a tiny pure function. AND
of two bits is 0 or 1. The interesting part is not the happy path (four rows of
a truth table); it is what the function does when handed something that is not a
bit: the number 2, the string `"1abc"`, `1.0`, `NaN`, `nil`. Every language
converts and compares values differently, so "reject anything that is not
exactly 0 or 1" has to be proved separately in each language.

## 2. Decision 1 — scope: the 11 basic gates

The cross-lane contract is these eleven operations, exactly as LG01 lists them
(LG01, "In scope", groups "Primitive gates" and "NAND-built gates"):

| Group | Operations (Warrant op names) |
|---|---|
| Primitive | `not_gate`, `and_gate`, `or_gate`, `xor_gate`, `nand_gate`, `nor_gate`, `xnor_gate` |
| NAND-built | `nand_not`, `nand_and`, `nand_or`, `nand_xor` |

Nine of them take two bits (`and`, `or`, `xor`, `nand`, `nor`, `xnor`,
`nand_and`, `nand_or`, `nand_xor`); two take one (`not_gate`, `nand_not`). This
split drives every count below (it is why there are 31 faults, not 33).

**Not in the cross-lane contract:** the n-input gates (`and_n`, `or_n`,
`xor_n`), the combinational blocks (`mux2`, `mux4`, `mux_n`, `demux`,
`decoder`, `encoder`, `priority_encoder`, `tri_state`) and everything
sequential. They stay Rust-only under LG01. Reasons: their inputs are byte
strings and lengths (the hard part of a port is the value mapping, which the
program wants to settle on the smallest possible surface first), several lanes
do not have them all (the audit in the backlog shard shows Haskell with six of
the eleven basic gates and no more), and the Rust claims for them need
`Bytes`-typed binders that other lanes' runners do not implement yet.

**Rationale for starting small.** Other packages are meant to "take the
proofs as assumptions" (owner direction, backlog shard). A small, finite,
fully enumerable contract is the one a downstream package can assume without
caveats in every language.

## 3. Decision 2 — a separate, corpus-only Warrant document

### 3.1 The decision

A new Warrant document `code/specs/logic-gates-basic.warrant`, module
`logic_gates_basic` (written by a later PR, item LGE-S1b), whose body is a
**byte-identical copy** of the corresponding blocks of `logic-gates.warrant`:

| Block kind | What is copied | Count | Source in `logic-gates.warrant` |
|---|---|---|---|
| `error` | `error GateError = INVALID_INPUT` | 1 | line 66 |
| `assume` | the two `assume` blocks (`platform`, `environment`) | 2 | lines 173-176 |
| `model` | `b2u` only (the Bool-to-wire helper the claims use) | 1 | line 72 |
| `op` | the eleven scalar operations | 11 | lines 150-160 |
| `labels` | one per op, all `public` | 11 | lines 184-214 |
| `input` | one per parameter (two-input gates 2, one-input gates 1: 9 x 2 + 2 x 1) | 20 | lines 251-329 |
| `bound` | one per op (`steps <= 1024`, `depth <= 64`) | 11 | lines 422-454 |
| `example` | every example whose subject is one of the eleven ops | 39 | lines 491-584 (rows for the other ops are interleaved and not copied) |
| `claim` | `<op>_truth_table`, `<op>_rejects_non_bit` for each op (22), plus the four `*_equals_*` claims | 26 | lines 666-719 |

The 39 examples are 26 truth-table rows (two for `not_gate`, four for each of
the six other primitives) and 13 rejection rows (counted by running a script
over the file, not by reading it). The 26 claims are 11 truth tables, 11
rejection claims and the four functional-completeness claims
(`nand_not_equals_not`, `nand_and_equals_and`, `nand_or_equals_or`,
`nand_xor_equals_xor`, lines 713-719).

Everything else in `logic-gates.warrant` (the 58 other models, the 11 non-scalar
ops, their `labels`/`input`/`bound`/`example`/`claim` blocks, and the long
header comment) is not copied. The new file gets its own short header comment
that points here.

The oracles inside the copied claims are the ones LG01 already reviewed: each
expected value is computed from Warrant's own `and`/`or`/`not`/`!=`/`==`
operators over `Bool`, never from the crate (header comment, lines 26-30 of
`logic-gates.warrant`). A lane therefore compares its gate against an oracle
that lives outside the lane (VER04i section 2.3).

### 3.2 Why not bind Rust to two documents

The assurance tooling assumes **one Warrant document per package**, pinned by
the package's directory name:

* `ca-assurance-report/src/evidence.rs:754-766` (`expected_spec`) computes the
  only acceptable spec path as `code/specs/<package directory name>.warrant`,
  "to stop a manifest from naming some other document".
* `ca-assurance-report/src/evidence.rs:1189-1214` derives the repository root
  and the package id (`rust/<name>`) from that same single path.
* `ca-assurance-report/src/evidence.rs:781` and `:854` read exactly one
  `<package>/evidence.bindings`.
* `warrant-grammar/examples/gen_evidence.rs:44-54` reads one
  `evidence.bindings` and calls `validate_paths(package)` so "the file cannot
  redirect the generator to another document".

Binding the Rust package to a second document would mean a second bindings file,
a second generated harness, a second manifest and a second profile in the
report, in a stack that has a single slot for each. That is a change to the
assurance tooling that has nothing to do with logic gates.

### 3.3 Why not an operation-subset profile

The other way to share one document is to keep `logic-gates.warrant` as the only
document and add a corpus profile that selects eleven of its 22 operations.
VER04i section 5 rule 1 ("Total") says that every `ops` entry in `corpus.json`
has a binding row, and every lane relies on it: a lane that binds *less* than
the corpus lists fails at start-up (`BindingInvalid`). An op-subset profile
would have to relax that rule (a lane may skip ops the profile excludes), and
every runner, the conformance kit and the report validator would then need to
distinguish "skipped by profile" from "forgotten by the binding author". That
weakens the one check that stops a binding from silently covering less than the
corpus, in order to save one 120-line file. A second document keeps rule 1
untouched.

### 3.4 The cost, stated

The second document duplicates about 120 lines of blocks. The duplication is
the only risk (the copies could drift apart), and section 4 removes it with a
mechanical check that fails the build.

## 4. Decision 3 — the drift guard

A standard-library-only Python script (written by a later PR, item LGE-S1b) at

```text
code/scripts/logic_gates_basic_drift.py
```

run from the Rust `logic-gates` BUILD (next to the existing
`gen_evidence --check` line, `code/packages/rust/logic-gates/BUILD:26`) and from
the fixture's own checks. It exits 0 on equality and 1 on any difference, and
prints the first differing block, never both files.

**What it compares, exactly.**

1. *Block extraction.* A block starts at a line indented by exactly two spaces
   whose first word is one of `error`, `assume`, `model`, `op`, `labels`,
   `input`, `bound`, `example`, `claim`, and continues over every following
   line that is blank or indented by more than two spaces. Lines whose first
   non-blank characters are `--` (comments) and blank lines are dropped before
   comparison; nothing else is normalised (no trimming, no whitespace folding,
   LF line endings required: a CR is a difference).
2. *Key set.* Each block has the key `(kind, name)` (for `input`, the dotted
   `op.param`). The script holds the expected key set itself: the 11 `op`
   names, 11 `labels`, the 20 `input` keys, 11 `bound`, the 39 `example` names
   found in the full document for those ops, the 26 `claim` names, the single
   `model` `b2u`, the `error` and the two `assume` blocks. The basic document
   must contain exactly this set: a missing block, an extra block (for example a
   `model junk`) and a duplicated block all fail.
3. *Block text.* For every key, the extracted text in `logic-gates-basic.warrant`
   equals the text in `logic-gates.warrant` byte for byte.
4. *Order.* Blocks of one kind appear in the same relative order in both files
   (claim order matters: a negative control stops at the first claim that
   catches its fault, `logic-gates.warrant` lines 659-660).
5. *Closure.* No block in the basic document names an operation outside the 11
   (so a copy of an `and_n` example is refused even if it is byte-identical).
6. *Headers.* The first non-comment line is `warrant logic_gates_basic v1` in
   the basic document and `warrant logic_gates v1` in the full one; this is the
   only structural difference allowed.
7. *Fault list.* The `fault` lines of
   `code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.profile`, grouped by
   op, equal the `faults=` lists on the 11 matching `op` lines of
   `code/packages/rust/logic-gates/evidence.bindings` (lines 50-60). The
   compiler's own `--bindings` comparison (VER04r section 8.4, last paragraph)
   cannot do this because the module names differ (`logic_gates_basic` versus
   `logic_gates`), which is why the profile is generated without `--bindings`
   (section 5).

The script has its own tests (a deliberately edited copy of each block kind must
fail), and the BUILD line fails closed if Python is not available.

## 5. Decision 4 — the corpus: `warrant-corpus-v2`

The corpus format is `warrant-corpus-v2` (VER04r), not v1. Reason: v1
generation is hard-wired to `ct-compare` (the lane-side evidence code is
bound to that one document; see section 6), whereas v2 generates a fixture from
any document and profile (VER04r section 1.1).

```text
code/specs/fixtures/logic-gates-basic-warrant-v2/
  corpus.json  chunks/*.txt  corpus.profile  cases.tfv
  consumers.json  schema.json  consumers.schema.json  README.md  CHANGELOG.md
```

(the layout is VER04r section 1.1; the directory name follows the existing
fixtures `ct-compare-warrant-v2` and `wasm-leb128-warrant-v2`, which use the
hyphenated package name, not the module name).

### 5.1 The profile: 31 faults

`corpus.profile` is hand-written, in the style of
`code/specs/fixtures/ct-compare-warrant-v2/corpus.profile`, and lists the
faults the Rust bindings already declare for these eleven operations
(`code/packages/rust/logic-gates/evidence.bindings:50-60`):

| Operations | Faults per op | Ops | Lines |
|---|---|---|---|
| `and_gate`, `or_gate`, `xor_gate`, `nand_gate`, `nor_gate`, `xnor_gate`, `nand_and`, `nand_or`, `nand_xor` | `missing-error`, `saturate-add`, `saturate-sub` | 9 | 27 |
| `not_gate`, `nand_not` | `missing-error`, `shift-mask` | 2 | 4 |
| | | 11 | **31** |

(The Rust bindings have 51 `fault` lines for all 22 operations, VER04r
section 13 "logic-gates adopter" row; 51 minus the 20 that belong to the other
eleven operations is 31.) The profile is generated **without `--bindings`**
(section 4 item 7 explains the replacement check). The three scalar faults
`saturate-add`, `saturate-sub` and `shift-mask` are `ir.fixed` templates
(VER04r section 8.2, rows for D51); their generator support is on `main`
(`feat(warrant-corpus-gen): saturate-add, saturate-sub, shift-mask ... (S11f-2)`,
commit `ee3de05f08a`).

Each control perturbs exactly one op (VER04r section 8.3) and passes only if a
required claim fails, so a lane cannot "prove" a gate by running a runner that
never calls it. The Rust BUILD already pins each fault to its cheap claim
(`code/packages/rust/logic-gates/BUILD:28-58`, one line per (op, fault): 31 lines, for example
`saturate-add` against `and_truth_table`); the generator recomputes those pairs
as `required_claims`.

### 5.2 Size: about 590 thousand cases in 26 suites

One suite per claim. The case counts, recomputed from the claim binders in
`logic-gates.warrant` lines 666-719:

| Suites | Binders | Cases each | Number | Cases |
|---|---|---|---|---|
| `<gate>_rejects_non_bit`, two-input gates | `a: U8, b: U8 \| a > 1 or b > 1` | 65,536 - 4 = 65,532 | 9 | 589,788 |
| `not_rejects_non_bit`, `nand_not_rejects_non_bit` | `a: U8 \| a > 1` | 256 - 2 = 254 | 2 | 508 |
| `<gate>_truth_table`, two-input gates | `a: Bool, b: Bool` | 4 | 9 | 36 |
| `not_truth_table`, `nand_not_truth_table` | `a: Bool` | 2 | 2 | 4 |
| `nand_and/or/xor_equals_*` | `a: Bool, b: Bool` | 4 | 3 | 12 |
| `nand_not_equals_not` | `a: Bool` | 2 | 1 | 2 |
| | | | **26** | **590,350** |

Plus the 39 examples, which live in `cases.tfv` and are not suites. Every
rejection suite has a product space of exactly 65,536 ordinals, so with the
default `chunk_size` of 65,536 (VER04r section 5.4) each is one chunk.

**Budgets.** The largest single claim is 65,536 product ordinals. That is below
VER04i section 3's per-claim `EXHAUSTIVE_BUDGET = 400,000` and far below the
v2 default `budget 16777216` (VER04r section 5.3, also the value in
`evidence.bindings:48`). The `max_ir_cost` default of 2,000 (VER04r section 8.4)
is expected to be enough, because these claims are one-line comparisons (the
Rust plan's 2,372 comes from the multiplexer claims that are not in this
document); the generator measures it, and the profile gets a `max_ir_cost` line
only if it must (this is stated as an expectation, not a measurement).

Because the domains are finite and fully enumerated, the honest label a lane may
print is the one VER04i section 13.3 uses for `gf256`: "exhaustive over the
whole domain".

## 6. Decision 5 — the dependency, stated honestly

**No lane can replay a v2 corpus today.** Concretely, what is not on `main`:

* **Python.** The `warrant-corpus` runner accepts only the v1 profile
  (`code/packages/python/warrant-corpus/src/coding_adventures_warrant_corpus/corpus.py:438`,
  `schema_version == 1 and profile == PROFILE`). The v2 runner is CV2 slices
  S6b/S6c (`VER-BACKLOG.d/portable-warrant.md`, row "CV2-S1 to CV2-S13"; "S5 to
  S7 are not on `main`", VER04r section 13).
* **TypeScript** (CV2-S7), **Go** (it "finishes on v1 and is ported to v2 with
  S6/S7", same backlog row) and every other lane: no v2 runner.
* **The report.** `ca-assurance-report` reads only the v1 lane profile and only
  ct-compare. `lane_evidence.rs:23-25` hard-wires `CORPUS_DIR`, `CORPUS_FILE`
  and `SPEC_FILE` to `ct-compare`; `:192` demands schema
  `warrant-lane-evidence-v1`; `:244` fails unless there are exactly 29 manifest rows (the
  constant `MAX_MANIFEST_ROWS` is at `:22`, checked at `:243`); `:364-369` rejects any module other
  than `ct_compare`; `:622-656` demands exactly 12 examples and 9 suites; `:730`
  and `:748` demand exactly 8 transformers and 8 controls. The generalisation is
  the CV2 slice S9 ("v2 lane validator in `ca-assurance-report`") plus a
  loader that reads the corpus path from the registry instead of from
  constants. Until both land, `ca-assurance-report --check-lane <lane>/logic-gates`
  cannot pass for any lane.

**What a release may truthfully claim until then.** Each lane's evidence is its
own exhaustive **lane-local** suite, labelled "lane-local exhaustive": literal
truth tables (expected values written out, not computed with the operator under
test), the full non-bit enumeration over the lane's integer type, and the
type-confusion matrix of section 9. It is real evidence over the real domain,
but its oracle lives in the lane's own test file, not outside it, and no
negative-control (sabotage) run is replayed against it. The report words stay
"lane-local exhaustive", never "contract evidence"; the label changes only when
a lane's runner replays the committed corpus and the report validator accepts
its v2 manifest.

**Order of lanes once the dependency clears** is VER04i section 13.2, as
amended by section 11 below: logic gates first in each lane, in place of
`ct-compare`.

## 7. Decision 6 — the error model, lane by lane

**Rule: bindings map a stable error identity, never a message.** VER04i section
5 rule 2 and VER04r section 7.5 require closed, exact matching: a raised class
(`type(e) is cls`), a Go sentinel by identity, a C status code, a `Left`
constructor or a `panics` row. An error that matches no row is `unexpected-error`
and fails the lane. Message text is never read.

`GateError` has a single case, `INVALID_INPUT` (`logic-gates.warrant` line 66),
so a binding uses the single-case forms (`error=INVALID_INPUT`, or `err_any`,
VER04r section 7.5), which is the same mechanism the Rust binding uses
(`evidence.bindings:11-18`).

**Decision:** every lane's library must expose a stable identity for the
rejection. Today sixteen lanes have a package (Rust and fifteen others); this is what each does and what
the binding row is. "Read at" is the file and line of the validator.

| # | Lane | Rejection idiom today | Read at | Binding row | Stable today? |
|---|---|---|---|---|---|
| 1 | rust | `assert!` panic | `rust/logic-gates/src/gates.rs:52-57` | `error=INVALID_INPUT` (generated adapter contains the panic; the `panics` row) | yes (this is the reference) |
| 2 | python | `ValueError` for an int outside {0,1}; `TypeError` for a non-int, a `bool` or a float | `python/logic-gates/src/logic_gates/gates.py:57-62` | `errors=INVALID_INPUT:ValueError` | yes |
| 3 | typescript | `RangeError` for a number outside {0,1}; `TypeError` for a non-number | `typescript/logic-gates/src/gates.ts:77-82` | `errors=INVALID_INPUT:RangeError` | yes |
| 4 | go | `panic(fmt.Sprintf(...))`, a plain string | `go/logic-gates/gates.go:90-94` | reviewed adapter recovers the panic, returns sentinel `ErrInvalidBit` (VER04i section 5.3) | **no**, see below |
| 5 | ruby | `ArgumentError` outside {0,1}; `TypeError` for a non-Integer | `ruby/logic_gates/lib/coding_adventures/logic_gates/gates.rb:90-98` | `errors=INVALID_INPUT:ArgumentError` | yes |
| 6 | c | none yet; package to be added | | `status=-1` (`LG_INVALID`) with an out parameter | to be built so |
| 7 | cpp | none yet; package to be added | | `errors=INVALID_INPUT:std::invalid_argument` | to be built so |
| 8 | java | `IllegalArgumentException` | `java/logic-gates/src/main/java/com/codingadventures/logicgates/LogicGates.java:70-74` | `errors=INVALID_INPUT:java.lang.IllegalArgumentException` | yes |
| 9 | kotlin | `require(...)`, which throws `IllegalArgumentException` | `kotlin/logic-gates/src/main/kotlin/com/codingadventures/logicgates/LogicGates.kt:68-72` | `errors=INVALID_INPUT:IllegalArgumentException` | yes |
| 10 | csharp | `ArgumentOutOfRangeException` | `csharp/logic-gates/LogicGates.cs:8-14` | `errors=INVALID_INPUT:System.ArgumentOutOfRangeException` | yes |
| 11 | fsharp | `ArgumentOutOfRangeException` | `fsharp/logic-gates/LogicGates.fs:8-10` | same class as csharp | yes |
| 12 | elixir | `ArgumentError` for a boolean, a non-integer or an integer outside {0,1} | `elixir/logic_gates/lib/coding_adventures/logic_gates/gates.ex:56-70` | `errors=INVALID_INPUT:ArgumentError` (one class covers every cause, which is fine for a single-case error) | yes |
| 13 | lua | `error(string)` | `lua/logic_gates/src/coding_adventures/logic_gates/gates.lua:72-76` | see below | **no** |
| 14 | perl | `die "<string>\n"` | `perl/logic-gates/lib/CodingAdventures/LogicGates.pm:100-104` | see below | **no** |
| 15 | haskell | `Left String` (an `Either`, not a raise) | `haskell/logic-gates/src/LogicGates/Basic.hs:14-17` | see below | **no** |
| 16 | swift | `throw LogicGateError.invalidBit(name:got:)` | `swift/logic_gates/Sources/LogicGates/Gates.swift:53-56` | `errors=INVALID_INPUT:invalidBit` | yes |
| 17 | dart | `ArgumentError.value(...)` | `dart/logic-gates/lib/src/gates.dart:5-9` | `errors=INVALID_INPUT:ArgumentError` | yes |
| 18 | ocaml | `Error (Invalid_bit {name; value})` in a `result` | `ocaml/logic-gates/src/coding_adventures_logic_gates.ml:7-17` | `returns_error=Invalid_bit` | yes |

Notes the table cannot hold:

* **Exact class matching.** `ArgumentOutOfRangeException` is a subclass of
  `ArgumentException`, and Python/Ruby have both a type error and a value error
  for the same gate. The runner matches the class that was raised, not a base
  class (VER04r section 7.5). The corpus input type is `U8` (an integer), so the
  corpus only ever reaches the *range* error; the type-error class is reached
  by the lane-local matrix (section 9) and is asserted there.
* **Go.** `gates.go:92` panics with a formatted string, and the same package
  panics with a string for an unrelated internal failure (`gates.go:154`,
  "AND CMOS evaluation error"). A recover-everything adapter would map both to
  the sentinel, which is exactly the vacuous pass VER04i section 5.3 forbids
  ("re-panicking any other panic"). **Decision:** the adapter is reviewed and
  returns `ErrInvalidBit` only for a panic whose value is the package's typed
  bit-rejection value; this needs the library to panic with a typed value
  instead of a string. That library change is a separate follow-up item
  (LGE-R, Go row), and until it lands the Go lane is not bindable.
* **Lua, Perl, Haskell must gain a stable identity** (each a separate
  follow-up item, because a binding may not match message text):

  | Lane | Required identity |
  |---|---|
  | lua | `error` raises a table with a `code` field (`"INVALID_INPUT"`), not a string; an internal failure gets a different `code`, so it stays `unexpected-error` |
  | perl | a blessed exception class `CodingAdventures::LogicGates::Error` with a `code` method; `die` throws an object, not a string |
  | haskell | a `LogicGateError` algebraic data type (constructor `InvalidBit` and the rest) replaces `String` in `Left` |

  Of the three, Haskell already returns a value rather than raising; the
  decision for it is the type of the `Left` payload, not the mechanism.
* **Rust and Go** panics are "errors" only inside the adapter that contains
  them. The library contract is the panic.

## 8. Decision 7 — the value mapping

Bits cross the boundary as the **lane's own integer type**, as the libraries
already take them: Rust `u8`, Go `int` (`gates.go:90`), Java `int`
(`LogicGates.java:70`), Kotlin `Int` (`LogicGates.kt:68`), C# `int`
(`LogicGates.cs:8`), Dart `int` (`gates.dart:2-6`), Haskell `Int`
(`Basic.hs:12`), OCaml `int` (`coding_adventures_logic_gates.ml:1`), Python,
Ruby, Elixir, Lua, Perl and JavaScript's number. The corpus type is Warrant
`U8`; the runner range-checks every corpus value on the way in (VER04r section
7.2: a value the host type cannot hold is never truncated), and an adapter may
translate representation and nothing else (VER04i section 5 rule 5). So the Go
adapter converts the corpus's `uint8` to `int`, and OCaml's native `int` is fine
because the rule only forbids it for widths over 32 bits (VER04r section 7.2).

**Python binding amendment needed.** VER04i section 5.1 says Python accepts only
`Bool=bool`, `Bytes=bytes`, `U64=int`, `Nat=int` in `values` and refuses
anything else as `BindingInvalid`. The basic gates need **`U8=int`**: a result is
checked with `type(x) is int` (so a `bool` is refused, as for `U64`) and the
value is range-checked to 0..255 at the boundary. TypeScript (`U8=number`) and
Go (`U8=uint8`) already list `U8` (VER04i section 5.2 and section 5.3 `values`
rows). The Python amendment belongs in VER04i section 5.1 when the Python v2
runner (CV2-S6b/S6c) lands; it is recorded here so the runner slice includes it.

## 9. Decision 8 — the type-confusion matrix (mandatory lane-local evidence)

The corpus has one input type, `U8`. For a dynamically typed lane it therefore
**cannot express a wrong-type input**: no corpus case passes a string, a float
or a list. The wrong-type behaviour of a dynamic lane is covered only by a
lane-local **type-confusion matrix**, mandatory for the lanes whose language lets
the call site pass anything (python, typescript, ruby, elixir, lua, perl, and
the dynamic call path of dart). The statically typed lanes (rust, go, c, cpp,
java, kotlin, csharp, fsharp, haskell, swift, ocaml) have the type system do this
work for the rows that cannot be written; they still run the wide-integer rows.

**Expected result of every row:** the call is rejected with the lane's
identity from section 7 (the *type* identity where the lane has one, for
example Python `TypeError`), it returns no value, and no input is modified. The
matrix is run on **all eleven operations**, with the bad value in **each
argument position**, with the other argument valid.

Minimum rows (a lane adds its own pitfalls in its per-language threat list):

| Group | Rows |
|---|---|
| Strings that look numeric | `"0"`, `"1"`, `"1abc"`, `"1 "`, `" 1"`, `"1\n"`, `"0x1"`, `"1e0"`, `""` |
| Booleans | `true`, `false` (Python `True` is an `int` subclass, `gates.py:42-49` rejects it deliberately) |
| Absence | `nil` / `undef` / `None` / `null` / `undefined`; an Elixir atom such as `:ok` |
| Floats | `1.0`, `0.0`, `-0.0`, `0.5`, `NaN`, `+Inf`, `-Inf` |
| Wide and negative integers | `-1`, `2`, `255`, `256`, `257`, `2^31`, `2^32`, `2^32 + 1`, `2^53 + 1`, `2^63`, `2^64`, `2^64 + 1`, and the lane's `INT_MIN`/`INT_MAX` (the truncation rows: a lane that reduces its input modulo 256 or masks the low bit would accept `256` or `257`) |
| Containers | empty list/array, `[0]`, `[1]`, `[[1]]`, empty map/object/dict, a function, a symbol, a closure |
| Lane-specific | Perl: a `dualvar` whose number is 1 and string is `"abc"`, a reference, an object with overloaded `==`; JavaScript: `1n`, `new Number(1)`, `[1]`, an object whose `valueOf` returns 1; Python: an `int` subclass instance; Ruby: `Rational(1, 1)`, `Float::NAN`; Lua: a string coerced by arithmetic |
| Short-circuit | `and`, `nand`, `nand_and`, `or`, `nor`, `nand_or`: the first argument valid and decisive (0 for AND, 1 for OR) with the second invalid, and the reverse; a gate must not return before validating both |

Two of these rows are known to need a decision rather than a reject, and the
matrix records the answer so it is reviewed rather than accidental:

* JavaScript `-0`. `gates.ts:80` is `value !== 0 && value !== 1`, and
  `-0 !== 0` is false, so `-0` is accepted today. Decision for the matrix: it
  is the number zero and may be accepted, but the result must never be `-0`
  (checked with `Object.is`), and the corpus-side rule that a binding refuses
  `-0` (VER04r section 7.2, `typescript`/`dart (web)` row) is unchanged.
* Lua `1.0`. The backlog audit records that the Lua library accepts `1.0`; the
  matrix row is expected to reject, and the fix is part of the Lua identity
  follow-up in section 7.

**Weakness mapping.** Each row maps to a class in `weakness-registry-v2` (spec to
follow, item LGE-S3; the owner added these three to the registry for logic
gates): **CWE-20** improper input validation (a non-bit accepted; the
short-circuit rows; the wide-integer rows), **CWE-843** type confusion (a value
of one type treated as another: numeric strings, containers, booleans, `dualvar`,
objects with `valueOf`) and **CWE-704** incorrect type conversion (coercions:
`"1abc"` read as 1, `1.0`/`-0.0` read as an integer, `256` truncated to 0,
`NaN` compared). The row-to-CWE assignment is provisional until the registry
spec lands; the matrix itself does not wait for it.

## 10. Findings that bear on the decisions

Recorded so the decision stays reviewable; none changes a decision above.

1. **A full-document Rust adopter is already planned.** VER04r section 13
   ("logic-gates adopter" row) plans `code/specs/fixtures/logic-gates-warrant-v2/`
   for all 22 operations (51 faults, about 3.77 million cases, largest suite
   1,048,576). That is compatible with this document: the basic corpus is a
   separate, smaller fixture with a different module name. The full adopter
   remains Rust-only and still needs S11c-2.
2. **Go cannot be bound until its panic is typed** (section 7).
3. **Haskell returns `Left String`**; it does not raise (section 7). The
   decision's wording "raise free-text strings" is about identity, which still
   applies.
4. **VER04r section 1.1 names the fixture directory `<module>-warrant-v2`**, but
   the shipped fixtures use the hyphenated package name; this document follows
   the shipped ones.
5. **Rust's own binding is not replayed from this corpus.** Rust keeps its
   generated harness from `logic-gates.warrant`; the corpus is for the other
   lanes (and for a later Rust "second implementation" check, VER04i section
   13.2 step 0).

## 11. Follow-up items

| ID (proposed) | Work |
|---|---|
| LGE-S1b | `logic-gates-basic.warrant` and `code/scripts/logic_gates_basic_drift.py` with tests, BUILD line in `rust/logic-gates` |
| LGE-S1c | `logic-gates-basic-warrant-v2` fixture and profile (needs the generator on `main`: it is) |
| LGE-S3 | `weakness-registry-v2` and the row-to-CWE mapping of section 9 |
| LGE-R-go | Go: typed panic value for the bit rejection |
| LGE-R-lua, LGE-R-perl, LGE-R-haskell | stable error identity (section 7 table) |
| CV2-S6b/S6c, S7, Go v2 port, S9 | the dependency of section 6 (owned by `portable-warrant.md`) |

## 12. Related specs

[LG01](LG01-logic-gates-contract.md) (the Rust contract and the full
document), [10-logic-gates.md](10-logic-gates.md) (semantics and pedagogy),
[VER04i](VER04i-polyglot-warrant-evidence.md) (polyglot evidence; its section
13.2 is amended by the same change as this document),
[VER04r](VER04r-warrant-corpus-v2.md) (the v2 corpus),
`VER-BACKLOG.d/portable-warrant.md` (CV2 slice status) and
`VER-BACKLOG.d/logic-gates-everywhere.md` (the program queue).
