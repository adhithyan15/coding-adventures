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

**Inputs and safety.** The script takes **no path arguments** and no
environment input. It resolves its own location with `realpath` to find the
repository root, and reads only these fixed repo-relative paths:
`code/specs/logic-gates.warrant`, `code/specs/logic-gates-basic.warrant`,
`code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.profile` and
`corpus.json`, `code/packages/rust/logic-gates/BUILD` and
`code/packages/rust/logic-gates/evidence.bindings`. For every input it (a) takes
the `realpath` and fails unless that path is still strictly under the root (this
covers a symbolic link in a *parent* directory as well as in the file itself),
(b) fails if the `lstat` mode is not a regular file, (c) opens it with
`O_NOFOLLOW` (or the platform's equivalent; on Windows, where `O_NOFOLLOW` does
not exist, the `lstat` and `realpath` checks carry the rule) and reads from that
descriptor, and (d) fails if the file is larger than **1 MiB** (1,048,576 bytes;
the largest input today, `logic-gates.warrant`, is 867 lines) or missing. A
guard that could be pointed at another file, or that followed a link out of the
repository, could be made to compare a file with itself.

### 4.1 The grammar facts the guard is built on

The guard parses as the Warrant grammar does, not by its own idea of the format:

* **Top-level keywords.** `code/grammars/warrant/warrant.grammar:3-4`: a file is
  the `warrant NAME [NAME]` line (column 0) followed by an indented body of
  declarations, and a declaration is exactly one of `model`, `error`, `op`,
  `example`, `claim`, `assume`, `labels`, `input`, `bound` (nine keywords). The
  keywords used *inside* blocks (`platform`, `environment`, `none`, `public`,
  `secret`, `hostile`, `rejects`, `with`, `ensures`, `fresh`, ...) are listed
  in the `keywords:` section of `code/grammars/warrant/warrant.tokens` and
  appear only below a declaration.
* **Comments.** `code/grammars/warrant/warrant.tokens:3-6` (`mode: indentation`):
  `COMMENT = /--[^\n]*/` and `WHITESPACE = /[ \t]+/` are both `skip` tokens. So a
  comment is `--` to the end of the line **wherever it begins**, at any
  indentation and after code on the same line, and the lexer never treats it as
  content.

### 4.2 What it compares, exactly

1. *Strict file shape.* Fail on any CR, any tab character anywhere (the lexer's
   `WHITESPACE` accepts tabs, but a tab makes indentation depth ambiguous, so a
   tab is a difference), a byte-order mark, invalid UTF-8, a missing final LF,
   and any indentation that is not a multiple of two spaces.
2. *Comment rule = the grammar's rule, applied outside literals.* The guard
   scans each line left to right the way the lexer does, with the lexer's token
   set. **The Warrant grammar has no general string literal.** The only quoted
   token in `code/grammars/warrant/warrant.tokens` is
   `BYTES_LITERAL = /b"(?:[0-9a-f]{2})*"/` (line 14): its body is hex digits
   only, so it can never contain `-`, a space or a quote. At each position the
   scan therefore does one of three things: (i) if the text at the position
   matches `BYTES_LITERAL`, it consumes the literal whole and keeps it as
   content; (ii) otherwise, if the text is `--`, it removes `--` to the end of
   the line (the `COMMENT` skip token, `warrant.tokens:5`, `/--[^\n]*/`); (iii)
   otherwise, a `"` that is not the start of a `BYTES_LITERAL` is **refused**
   (there is no token for it, so a quoted string is a lexer error in the
   Warrant, and a comment-looking ` -- ` inside quotes cannot be smuggled past
   the guard as a "string"). Then trailing blanks are dropped, and a line empty
   after that is blank. Nothing else is normalised. A `--` immediately preceded by
   a non-blank character (as in `a--b`) is refused, because the lexer's choice
   between `COMMENT` and two `MINUS` tokens there is not something a spec should
   depend on. Quotes that appear **inside a comment** (the header of
   `logic-gates.warrant` has several, for example line 18) are comment text and
   are ignored with it. Checked against the current file: after comments are
   removed, every `"` in `logic-gates.warrant` begins a `BYTES_LITERAL`.
   The refuse-any-quote alternative (refuse every `"` that is not a
   `BYTES_LITERAL`) is exactly what rule (iii) does, so no separate string
   tokeniser is needed and the two readings agree. An indented `--` line is
   skipped only because the grammar skips it.
3. *No line outside a block (closes the vacuous pass).* After step 2, every
   non-blank line must be either (a) the single column-0 header line
   `warrant <module> v1`, or (b) inside an extracted block. A column-0 line that
   is not the header fails. A two-space line whose first word is not one of the
   nine declaration keywords fails. A line indented by more than two spaces with
   no open block fails. So text the extractor does not recognise can never hide
   from the comparison: it is a difference, not a skip.
4. *Block extraction.* A block starts at a two-space line whose first word is a
   declaration keyword and continues over every following line indented by more
   than two spaces (blank lines do not end it).
5. *Key set.* Each block has the key `(kind, name)` (for `input`, the dotted
   `op.param`; for `assume`, `platform` or `environment`). The script holds the
   expected key set itself: the 11 `op` names, 11 `labels`, the 20 `input` keys,
   11 `bound`, the 39 `example` names found in the full document for those ops,
   the 26 `claim` names, the single `model` `b2u`, the `error` and the two
   `assume` blocks. The basic document must contain exactly this set: a missing
   block, an extra block (for example a `model junk`) and a duplicated block all
   fail.
6. *Block text.* For every key, the block text (after step 2) in
   `logic-gates-basic.warrant` equals the text in `logic-gates.warrant` byte for
   byte.
7. *Order.* Blocks of one kind appear in the same relative order in both files
   (claim order matters: a negative control stops at the first claim that
   catches its fault, `logic-gates.warrant` lines 659-660).
8. *Closure.* No block in the basic document names an operation outside the 11
   (so a copy of an `and_n` example is refused even if it is byte-identical).
9. *Headers.* The header line is `warrant logic_gates_basic v1` in the basic
   document and `warrant logic_gates v1` in the full one; this is the only
   structural difference allowed.
10. *Fault list, per (op, fault).* The `fault` lines of
    `code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.profile`, grouped by
    op, equal the `faults=` lists on the 11 matching `op` lines of
    `code/packages/rust/logic-gates/evidence.bindings` (lines 50-60). The
    compiler's own `--bindings` comparison (VER04r section 8.4, last paragraph)
    cannot do this because the module names differ (`logic_gates_basic` versus
    `logic_gates`), which is why the profile is generated without `--bindings`
    (section 5).
11. *Fault pins, per (op, fault, required claim).* Fault names alone are not
    enough: a fault is only evidence if some claim catches it. The Rust BUILD pins
    one claim per fault in lines of the form
    `WARRANT_EVIDENCE_FAULT=<fault> WARRANT_EVIDENCE_FILTER=/exhaustive/<claim> ...`
    (`code/packages/rust/logic-gates/BUILD:28-58`). The guard parses those 31
    lines for the eleven ops and maps each claim to its op with an **explicit
    claim-to-op table embedded in the script**, not by inspecting the claim body:
    for each of the 11 ops `<claim stem>_truth_table` and
    `<claim stem>_rejects_non_bit` map to that op (stems: `not`, `and`, `or`,
    `xor`, `nand`, `nor`, `xnor` map to `not_gate`, `and_gate`, ... and
    `nand_not`, `nand_and`, `nand_or`, `nand_xor` to themselves), and the four
    `*_equals_*` claims are not pinned by any fault today; if one ever is, a claim
    that mentions two ops (for example `nand_xor_equals_xor`, which calls
    `nand_xor` and `xor_gate`) maps to the **NAND-built** op (`nand_xor`), the
    one whose behaviour it exercises. The table is itself checked: its claim
    names must equal the 26 claim names of check 5, so a renamed claim cannot
    leave a stale entry. The guard compares with the committed `corpus.json`: the
    set of `(op, fault)` pairs is equal, and for every pair the BUILD's pinned
    claim is a member of that control's `required_claims` (VER04r section 8.3,
    control row). A pin the corpus does not list, a control with no pin, and a
    pinned claim the corpus calls blind all fail.
12. *Fixture freshness.* The fixture must have been generated from the current
    basic document. The generator already enforces this: `corpus.json` records
    `spec_sha256`, the SHA-256 of the spec's raw bytes
    (`code/packages/rust/warrant-corpus-gen/src/emit.rs:444`), and its `--check`
    regenerates the corpus and compares it byte for byte with the committed
    directory (`code/packages/rust/warrant-corpus-gen/src/cli.rs:10`), so editing
    the document without regenerating the fixture is a `--check` drift. The guard
    additionally recomputes the SHA-256 of `logic-gates-basic.warrant` and fails
    if it differs from `corpus.json`'s `spec_sha256`, so the failure is reported
    here with a clear message and not only by the generator. **Honest status of the generator check:** a generator `--check` is
    wired in a BUILD today only for `ct-compare`
    (`code/packages/rust/warrant-corpus-gen/BUILD:25` generates that fixture into
    a temporary directory, runs `--check`, damages one chunk and requires the
    check to fail). It is not run for this fixture yet. LGE-S1c adds the same
    kind of line for `logic-gates-basic` to that BUILD (without `--bindings`,
    section 5.1). Until it lands, **the guard's SHA-256 comparison is the only
    freshness check**, and it proves that `corpus.json` was generated from the
    current document, not that the rest of the fixture matches `corpus.json`.

The script has its own tests in
`code/scripts/tests/test_logic_gates_basic_drift.py` (the repository's
convention for script tests is `code/scripts/tests/test_<script>.py`; for example
`test_warrant_negctl.py` tests `code/scripts/warrant_negctl.py`). A deliberately
edited copy of each block kind must fail, and so must each of these: a stray
column-0 line, a two-space line with an unknown keyword, a tab-indented line, a
one-byte change inside a copied claim, **a one-byte difference inside a
`BYTES_LITERAL`** (the only quoted content the grammar has), **a line containing a
plain quoted string with ` -- ` inside it** (refused by rule (iii), proving the
comment rule cannot be hidden behind quotes), a symbolic link in place of an
input, a symbolic link in a parent directory, and an input over 1 MiB. An
indented `--` comment line that changes nothing else must pass. The BUILD line
fails closed if Python is not available.

### 4.3 When the guard runs (build-tool selection)

The three compared files live under `code/specs/`, outside the package directory
`code/packages/rust/logic-gates`, and the build tool selects packages by changed
path. Today an edit to `logic-gates.warrant` already selects `rust/logic-gates`:
`warrantEvidenceRow("logic-gates")` is listed at
`code/programs/go/build-tool/repository_inputs.go:601`, and the function (at
`:1773`, introduced by commit `1a40582120e`, CV2-F13) matches the exact path
`code/specs/<crate>.warrant`. It does not select the package for an edit to the
*basic* document, the fixture or the drift script, so the guard could be skipped
in the very PR that breaks it. Decision: extend that mechanism with one more
repository-input row for `rust/logic-gates` whose `matches` function accepts
exactly these paths and whose `paths` list names them:

* `code/specs/logic-gates.warrant` (already covered; listed so the row is
  self-contained),
* `code/specs/logic-gates-basic.warrant`,
* `code/specs/fixtures/logic-gates-basic-warrant-v2/` (any file under it),
* `code/scripts/logic_gates_basic_drift.py` and its test file.

The fixture is also added to the `warrantAdopters` table
(`repository_inputs.go:398`; `:389-397` explains it) so the generator and the
oracle are selected for it too. Tests, in the style of
`code/programs/go/build-tool/warrant_evidence_spec_inputs_test.go`:

1. editing `logic-gates-basic.warrant`, any fixture file, or the drift script
   selects `rust/logic-gates`, and a lookalike path
   (`logic-gates-basic.warrant.draft`, `logic-gates-basic2.warrant`) selects
   nothing;
2. `TestWarrantEvidenceRowsCoverEveryEvidenceCrate` (`:139` of that test file)
   still passes, and a new test pins the new row to the BUILD line that runs the
   drift script;
3. a drift test for the script: copy the two documents to a temporary tree, flip
   one byte inside a copied claim, and require exit 1; restore it and require
   exit 0.

## 5. Decision 4 — the corpus: `warrant-corpus-v2`

The corpus format is `warrant-corpus-v2` (VER04r), not v1. Reason: v1
generation is hard-wired to `ct-compare` (the lane-side evidence code is
bound to that one document; see section 6), whereas v2 generates a fixture from
any document and profile (VER04r section 1.1).

```text
code/specs/fixtures/logic-gates-basic-warrant-v2/
  corpus.json  chunks/*.txt  corpus.profile  cases.tfv
  README.md  .gitattributes  (no consumers.json: see Errata LGE-S1c)
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

**The label names the domain.** The corpus input type is `U8`, so the honest
label is "**exhaustive over U8 (0..255)**" (for a two-input gate, over
U8 x U8, 65,536 argument pairs); it is not "exhaustive over all inputs". Wider
integers are outside the corpus: `256` (a value that wraps to 0 in a byte),
`257` (wraps to 1), `2^32 + 1`, `2^64 + 1`, negative numbers and every
non-integer are **not covered by the corpus at all**. A lane that masks or
truncates its argument to 8 bits (or to its low bit) passes every corpus case
and is still wrong. That class is covered **only** by the lane-local matrix rows
of section 9 ("Wide and negative integers"), and a lane's evidence statement
must say so.

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
own exhaustive **lane-local** suite, labelled "lane-local exhaustive over U8
(0..255)" (the domain is part of the label, section 5.2): literal
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
rejection. All eighteen lanes now have a package and, as of the errata of
2026-11-24 (LGE-LG02-ERRATA, below), every row here was re-read from the merged
source on `origin/main`. "Read at" is the file and line of the validator or of
the identity's definition. **The binding row is the exact class** the lane's
runner matches (`type(e) is cls`, VER04r section 7.5); a subclass is not a
match and a base class is not a match.

| # | Lane | Rejection idiom on `main` | Read at | Binding row | Stable today? |
|---|---|---|---|---|---|
| 1 | rust | `assert!` panic | `rust/logic-gates/src/gates.rs:52-57` | `error=INVALID_INPUT` (generated adapter contains the panic; the `panics` row) | yes (this is the reference) |
| 2 | python | `ValueError` for an int outside {0,1}; `TypeError` for a non-int, a `bool`, an `int` subclass or a float | `python/logic-gates/src/logic_gates/gates.py:67-71` | `errors=INVALID_INPUT:ValueError` | yes (general-purpose class) |
| 3 | typescript | `RangeError` for an integer outside {0,1} or `-0`; `TypeError` for a non-number or a non-integer number | `typescript/logic-gates/src/gates.ts:85-93` | `errors=INVALID_INPUT:RangeError` | yes (general-purpose class) |
| 4 | go | panic with the typed value `*InvalidBitError` (an internal failure panics with a different type, `*InternalError`); `IsInvalidBit` is the exact-type test | `go/logic-gates/gates.go:115-118` (type), `:168-172` (panics) | reviewed adapter recovers the panic and returns sentinel `ErrInvalidBit` only when the value is `*InvalidBitError` (VER04i section 5.3) | yes (PR #19935) |
| 5 | ruby | `CodingAdventures::LogicGates::InvalidBitError < ArgumentError` for anything that is not the Integer 0 or 1 (PR #19950) | `ruby/logic_gates/lib/coding_adventures/logic_gates/gates.rb:100` (class), `:139-141` (raise) | `errors=INVALID_INPUT:CodingAdventures::LogicGates::InvalidBitError` | yes (dedicated class) |
| 6 | c | returns `LG_INVALID` (-1) from every gate (PR #19926) | `c/logic-gates/src/logic_gates.c:29`; `c/logic-gates/include/logic_gates.h:43` | `status=-1` (`LG_INVALID`) with an out parameter | yes |
| 7 | cpp | `throw std::invalid_argument` from `detail::reject` (PR #19928) | `cpp/logic-gates/include/logic_gates/logic_gates.hpp:77-79` | `errors=INVALID_INPUT:std::invalid_argument` | yes (general-purpose class) |
| 8 | java | `com.codingadventures.logicgates.InvalidBitException extends IllegalArgumentException` (PR #19937) | `java/logic-gates/src/main/java/com/codingadventures/logicgates/InvalidBitException.java:14`; thrown at `LogicGates.java:75` | `errors=INVALID_INPUT:com.codingadventures.logicgates.InvalidBitException` | yes (dedicated class) |
| 9 | kotlin | `com.codingadventures.logicgates.InvalidBitException : IllegalArgumentException` (PR #19938) | `kotlin/logic-gates/src/main/kotlin/com/codingadventures/logicgates/LogicGates.kt:501-502` (class), `:76` (throw) | `errors=INVALID_INPUT:com.codingadventures.logicgates.InvalidBitException` | yes (dedicated class) |
| 10 | csharp | `CodingAdventures.LogicGates.InvalidBitException : ArgumentOutOfRangeException` (sealed; PR #19939) | `csharp/logic-gates/LogicGates.cs:12` (class), `:48` (throw) | `errors=INVALID_INPUT:CodingAdventures.LogicGates.InvalidBitException` | yes (dedicated class) |
| 11 | fsharp | `CodingAdventures.LogicGates.InvalidBitException`, inheriting `ArgumentOutOfRangeException` (PR #19941) | `fsharp/logic-gates/LogicGates.fs:16-17` (type), `:30` (raise) | `errors=INVALID_INPUT:CodingAdventures.LogicGates.InvalidBitException` | yes (dedicated class) |
| 12 | elixir | `CodingAdventures.LogicGates.InvalidBitError` exception (`defexception [:param, :value_description]`; PR #19952) | `elixir/logic_gates/lib/coding_adventures/logic_gates/invalid_bit_error.ex:21`; raised at `gates.ex:69` | `errors=INVALID_INPUT:CodingAdventures.LogicGates.InvalidBitError` | yes (dedicated exception) |
| 13 | lua | `error(<table>, 0)` with `code = "INVALID_BIT"` (PRs #19936, #19957); an internal failure raises a different shape and stays `unexpected-error` | `lua/logic_gates/src/coding_adventures/logic_gates/gates.lua:91` (code), `:192` (raise); `gates.is_invalid_bit` at `:120` | `errors=INVALID_INPUT:INVALID_BIT` (the `code` field) | yes |
| 14 | perl | `die` of a `CodingAdventures::LogicGates::Error` object with `code` `INVALID_BIT` (PR #19944); `is_invalid_bit` is the exact-class test | `perl/logic-gates/lib/CodingAdventures/LogicGates/Error.pm:45` (codes), `:113` (`is_invalid_bit`); thrown at `LogicGates.pm:66-72,182` | `errors=INVALID_INPUT:INVALID_BIT` (the `code`) | yes |
| 15 | haskell | `Left (InvalidBit { param, value })` of the `LogicGateError` ADT (PR #19942); an `Either`, not a raise | `haskell/logic-gates/src/LogicGates/Basic.hs:52-56` (type), `:70` (the `Left`) | `returns_error=InvalidBit` | yes |
| 16 | swift | `throw LogicGateError.invalidBit(name:got:)` | `swift/logic_gates/Sources/LogicGates/Gates.swift:56` (case), `:140-142` (throw) | `errors=INVALID_INPUT:invalidBit` | yes |
| 17 | dart | `InvalidBitError extends ArgumentError` (PR #19953) | `dart/logic-gates/lib/src/gates.dart:9` (class), `:22` (throw) | `errors=INVALID_INPUT:InvalidBitError` | yes (dedicated class) |
| 18 | ocaml | `Error (Invalid_bit {name; value})` in a `result` | `ocaml/logic-gates/src/coding_adventures_logic_gates.ml:8,17` | `returns_error=Invalid_bit` | yes |

**Rejection-specific identity, and the residual risk of class-only matching.**
Where a library offers an identity that means "this was a bit rejection" and
nothing else, the binding must use it: OCaml's `Invalid_bit` constructor,
Haskell's `InvalidBit` constructor, Swift's `LogicGateError.invalidBit`, the
dedicated classes of Ruby, Java, Kotlin, C#, F#, Elixir, Dart and Go
(`*InvalidBitError`), and the `INVALID_BIT` code of Lua and Perl. Only Python,
TypeScript and C++ still bind a **general-purpose class** (`ValueError`,
`RangeError`, `std::invalid_argument`), because those libraries offer nothing
narrower and the runner matches the class that was raised and not the message
(VER04r section 7.5). Rust matches the panic. C matches the status code. The
**residual risk** for the general-purpose rows is that an unrelated failure of
the same class raised from inside a gate (for example from the transistor layer
the gates delegate to, which can fail) would be mapped to `INVALID_INPUT` and
look like a correct rejection. (Go's string-panic version of this risk is
closed: its internal failures panic with `*InternalError`,
`go/logic-gates/gates.go:133`.) Why the corpus still catches the harmful
direction: the truth-table and equivalence suites call every gate on every
*valid* input and expect `Ok(value)` (`logic-gates.warrant` lines 666-719), so a
misfire on a valid input becomes `Err(INVALID_INPUT)`, which differs from the
expected `Ok` and fails the claim. What is not caught is a misfire on an
*invalid* input for the wrong reason: the outcome (a rejection) is still the
required one, and the corpus domain cannot see the reason. That is the accepted
limit of class-only matching; a lane that wants more adds the dedicated class.

Notes the table cannot hold:

* **Exact class matching.** The dedicated classes are subclasses of a base
  class (`InvalidBitError < ArgumentError`, `InvalidBitException <
  IllegalArgumentException` / `ArgumentOutOfRangeException`, Dart
  `InvalidBitError extends ArgumentError`). The runner matches the class that
  was raised, not a base class (VER04r section 7.5), so a lane's runner must name
  the dedicated class: a binding that still says `ArgumentError`,
  `IllegalArgumentException` or `ArgumentOutOfRangeException` matches nothing
  and fails the lane (the pre-errata table said exactly that; that was the
  drift this errata removes). Python and TypeScript have both a type error and a
  value/range error for the same gate; the corpus input type is `U8` (an
  integer), so the corpus only ever reaches the *range* error
  (`ValueError`, `RangeError`); the type-error class is reached by the lane-local
  matrix (section 9) and is asserted there.
* **Go** (resolved by PR #19935, LGE-R-go). The library panics with the typed
  value `*InvalidBitError` (`gates.go:168-172`); the reviewed adapter returns
  `ErrInvalidBit` only for that type and re-panics everything else
  (VER04i section 5.3).
* **Lua, Perl, Haskell** (resolved by PRs #19936/#19957, #19944, #19942): Lua
  raises a table with `code = "INVALID_BIT"`; Perl dies with a
  `CodingAdventures::LogicGates::Error` object whose `code` is `INVALID_BIT`;
  Haskell returns `Left (InvalidBit ...)` of the `LogicGateError` ADT, so for it
  the identity is a constructor, not a raise.
* **Rust and Go** panics are "errors" only inside the adapter that contains
  them. The library contract is the panic.

### 7.1 Shape and size errors (distinct from the bad-bit identity)

A *shape* or *size* error is a container or width argument that is the wrong
length, size or kind (a 3-element `sel` for a 2-way mux, a counter width of 0,
`inputs` not a power of two, a one-hot encoder input that is not one-hot).
It is **not** a bad bit and must never carry the bad-bit identity, because the
bad-bit identity is what the basic corpus asserts as `INVALID_INPUT`.

**Matching rule for runners.** The **bad-bit identity is matched exactly**
(the table above). The **shape identity is matched by family**: the runner (or
the lane-local test) asserts only that the call was rejected with an error that
is *not* the bad-bit identity and *is* a member of the lane's shape family
below. The shape errors are outside the 11-gate basic corpus (which has only the
`U8` input and the single `INVALID_INPUT` case), so no corpus runner matches
them; they are asserted by lane-local tests. The family is deliberately loose
because several lanes still reject shape with a general-purpose class.

Status as of 2026-11-24 (`origin/main`). "Merged" cites the PR that landed it;
"planned" is not on `main` and no runner may rely on it.

| Lane | Shape / size identity on `main` | Read at | Status of a dedicated shape type |
|---|---|---|---|
| rust | `assert!` panic with a message | `rust/logic-gates/src/combinational.rs:186,227`, `sequential.rs:292,350` | none; the panic is the contract |
| python | `ValueError` (wrong length or width); `TypeError` (wrong container type) | `python/logic-gates/src/logic_gates/combinational.py:81-87`, `sequential.py:68-93` | none planned; family is `{ValueError, TypeError}` |
| typescript | `RangeError` (wrong length or width); `TypeError` (wrong container type) | `typescript/logic-gates/src/sequential.ts:102-107,140-144` | none planned; family is `{RangeError, TypeError}` |
| go | plain-string `panic(fmt.Sprintf(...))` | `go/logic-gates/combinational.go:143,180-197`, `gates.go:636-713` | **planned**: typed `SizeError` (LGE-GO-SIZEERR); until it lands the shape family is "a panic whose value is not `*InvalidBitError` and not `*InternalError`" |
| ruby | plain `ArgumentError` | `ruby/logic_gates/lib/coding_adventures/logic_gates/combinational.rb:131-366`, `gates.rb:575,591` | none planned. Note `InvalidBitError < ArgumentError`, so a shape check must test "`ArgumentError` and not `InvalidBitError`". `sequential.rb:76` still raises plain `ArgumentError` for a bad bit inside the sequential layer (outside the basic-gate scope; to be fixed with the sequential errata) |
| c | none: the basic package has no container arguments | | not applicable |
| cpp | none: the basic package has no container arguments | | not applicable |
| java | `IllegalArgumentException` (plain, `AND_N`/`OR_N` arity) | `java/logic-gates/src/main/java/com/codingadventures/logicgates/LogicGates.java:466,486` | **planned**: `InvalidShapeException`, local branch only, unmerged |
| kotlin | `require` -> `IllegalArgumentException` | `kotlin/logic-gates/src/main/kotlin/com/codingadventures/logicgates/LogicGates.kt:441,459` | **planned**: `InvalidShapeException`, local branch only, unmerged |
| csharp | `ArgumentException` (with `ParamName`) | `csharp/logic-gates/LogicGates.cs:145,168` | none planned. `InvalidBitException` derives from `ArgumentOutOfRangeException`, which derives from `ArgumentException`: a shape check must test "`ArgumentException` and not `InvalidBitException`" |
| fsharp | `invalidArg` -> `ArgumentException` | `fsharp/logic-gates/LogicGates.fs:98,106` | none planned; same caveat as C# |
| elixir | plain `ArgumentError` | `elixir/logic_gates/lib/coding_adventures/logic_gates/combinational.ex:136-395`, `gates.ex:537,564` | none planned; `InvalidBitError` is not an `ArgumentError`, so the families are disjoint |
| lua | `error(<table>)` with `code = "INVALID_SHAPE"` or `"INVALID_WIDTH"`; `"INVALID_ARITY"` for a wrong argument count | `lua/logic_gates/src/coding_adventures/logic_gates/gates.lua:131-132,169-175` | **merged** (PR #19957, with #19936); family is `{INVALID_SHAPE, INVALID_WIDTH, INVALID_ARITY}` |
| perl | `die` of a `CodingAdventures::LogicGates::Error` with code `INVALID_SHAPE`, `INVALID_WIDTH` or `INVALID_ARITY` | `perl/logic-gates/lib/CodingAdventures/LogicGates/Error.pm:45`, `LogicGates.pm:666,800,856` | **merged** (PR #19944) |
| haskell | `Left (InvalidShape String)` of `LogicGateError` | `haskell/logic-gates/src/LogicGates/Basic.hs:56`, `Combinational.hs:28-107` | **merged** (PR #19942) |
| swift | `LogicGateError.invalidWidth`, `.invalidShape`, `.invalidSelectLength`, `.insufficientInputs`, `.invalidEncoderInput` | `swift/logic_gates/Sources/LogicGates/Gates.swift:62-75`, `Combinational.swift:124-395` | merged (cases exist in the enum on `main`); family is every case other than `.invalidBit` |
| dart | plain `ArgumentError` (via `ArgumentError.value`) | `dart/logic-gates/lib/src/combinational.dart:14-123`, `sequential.dart:103-193` | none planned. `InvalidBitError extends ArgumentError`, so a shape check must test "`ArgumentError` and not `InvalidBitError`" |
| ocaml | `Error` with `Invalid_length`, `Invalid_width`, `Invalid_power_of_two`, `Invalid_one_hot` or `Invalid_counter_state` | `ocaml/logic-gates/src/coding_adventures_logic_gates.ml:9-13` | merged; family is every constructor other than `Invalid_bit` |

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
bad-bit identity from section 7 (the *type* identity where the lane has one, for
example Python `TypeError`; for Ruby, Elixir, Java, Kotlin, C#, F#, Dart, Go, Lua and Perl
the same dedicated identity covers both the type and the range cause), it returns no value, and no input is modified. The
matrix is run on **all eleven operations**, with the bad value in **each
argument position**, with the other argument valid.

Minimum rows (a lane adds its own pitfalls in its per-language threat list):

| Group | Rows |
|---|---|
| Strings that look numeric | `"0"`, `"1"`, `"1abc"`, `"1 "`, `" 1"`, `"1\n"`, `"0x1"`, `"1e0"`, `""` |
| Booleans | `true`, `false` (Python `True` is an `int` subclass, `gates.py:67-71` rejects it deliberately) |
| Absence | `nil` / `undef` / `None` / `null` / `undefined`; an Elixir atom such as `:ok` |
| Floats | `1.0`, `0.0`, `-0.0`, `0.5`, `NaN`, `+Inf`, `-Inf` (in JavaScript `1.0` and `0.0` are accepted, see the rule table below) |
| Wide and negative integers | `-1`, `2`, `255`, `256`, `257`, `2^31`, `2^32`, `2^32 + 1`, `2^53 + 1`, `2^63`, `2^64`, `2^64 + 1`, and the lane's `INT_MIN`/`INT_MAX` (the truncation rows: a lane that reduces its input modulo 256 or masks the low bit would accept `256` or `257`) |
| Containers | empty list/array, `[0]`, `[1]`, `[[1]]`, empty map/object/dict, a function, a symbol, a closure |
| Lane-specific | Perl: a `dualvar` whose number is 1 and string is `"abc"`, a reference, an object with overloaded `==`; JavaScript: `1n`, `new Number(1)`, `[1]`, an object whose `valueOf` returns 1; Python: an `int` subclass instance; Ruby: `Rational(1, 1)`, `Float::NAN`; Lua: a string coerced by arithmetic |
| Short-circuit | `and`, `nand`, `nand_and`, `or`, `nor`, `nand_or`: the first argument valid and decisive (0 for AND, 1 for OR) with the second invalid, and the reverse; a gate must not return before validating both |

**One accept/reject rule per lane.** The rule is the same idea everywhere:
*accept only the exact integer 0 or the exact integer 1 of the lane's integer
type; reject everything else.* The table states the rule per dynamic lane, and
then which matrix rows are **reject rows** and which are **accepted exceptions**,
so no row is ambiguous.

| Lane | Accepts | Reject rows (must raise the section 7 identity) | Accepted rows (not reject rows) |
|---|---|---|---|
| python | `type(x) is int` and 0 or 1 (`gates.py:67-71`) | everything else, including `True`/`False`, `1.0`, `0.0`, numeric strings | none |
| typescript | `typeof x === "number"` and `Object.is(x, 0)` or `Object.is(x, 1)` (`gates.ts:85-93`; `-0` has been rejected since PR #19932) | `-0`, `-0.0`, `NaN`, `+-Inf`, `0.5`, `2`, every string, `1n`, `new Number(1)`, arrays, objects with `valueOf`, `null`, `undefined` | `1.0` and `0.0` are the **same JavaScript values** as `1` and `0` (the language has one `number` type), so they are accepted and are not matrix reject rows. This is the one explicit JS exception |
| ruby | `Integer` and 0 or 1 (`gates.rb:139-141`; PR #19950) | `1.0`, `0.0`, `Rational(1, 1)`, `true`, `false`, strings, `nil`, arrays | none |
| elixir | `is_integer` and 0 or 1 (`gates.ex:62-69`; PR #19952) | `1.0`, `true`, `false`, atoms, `nil`, strings, lists | none |
| lua | `math.type(x) == "integer"` and 0 or 1 (Lua 5.3+, the runner already requires `math.type`, VER04r section 7.2 lua row) | `1.0`, `0.0`, `-0.0`, `NaN`, `+-inf`, strings (including `"1"`: Lua would coerce them in arithmetic), booleans, `nil`, tables | none. `gates.lua:186-192` tests `mtype(value) == "integer"`, so `1.0` is rejected (PR #19936) |
| perl | defined, not a reference, and its string form matches `/\A[01]\z/` (decided by LGE-P1, PR #19927). The strings `"0"`/`"1"` and the integers 0/1 are accepted, as are the floats 0.0/1.0, because Perl stringifies them as `"0"`/`"1"`. Validation returns a fresh canonical integer | `"1.0"`, `" 1"`, `"1\n"`, `"01"`, `"+1"`, `"0 but true"`, `"1abc"`, the empty string, a `dualvar` whose string part is not a bit, references, `undef`, `NaN`, `Inf` | the strings `"0"`/`"1"` and the floats 0.0/1.0 (the documented Perl exception: the language does not separate the number from its string form) |

**Gate rule: a failing row blocks the label.** A lane whose matrix has a row that
currently fails, or whose stable bad-bit identity (section 7) is not yet merged,
**may not claim the lane-local label as passing** until the library change that
fixes it has merged. The matrix is committed with the failing row marked, so the
gap is visible, but the lane's release text may say "lane-local exhaustive" only
when every row of its matrix passes. As of the 2026-11-24 errata every lane
named in the first version of this rule (TypeScript `-0`, Lua `1.0`, and the
Lua, Perl, Haskell and Go identities) has merged its fix, so the rule currently
blocks no lane on these grounds; it remains in force for any lane whose matrix
fails later.

The statically typed lanes accept the integers 0 and 1 and reject every other
value of their integer type; the booleans, floats and strings rows do not exist
there because the compiler refuses them before the call.

**Why `-0` matters.** In JavaScript `-0 === 0`, so a check written as
`value !== 0 && value !== 1` lets `-0` through as if it were a valid bit
(`gates.ts:91`). `Object.is` tells them apart. The corpus side already refuses
`-0` at the boundary for the same reason (VER04r section 7.2, `typescript, dart
(web)` row), so the library and the runner then agree on one rule.

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
2. **Go's panic is typed** (PR #19935); the Go lane is bindable (section 7).
3. **Haskell returns `Left (InvalidBit ...)`**; it does not raise (section 7). The
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
| LGE-S1d | build-tool: the `rust/logic-gates` input row, the `warrantAdopters` row and their tests (section 4.3) |
| LGE-S1c | `logic-gates-basic-warrant-v2` fixture and profile (needs the generator on `main`: it is) |
| LGE-S3 | `weakness-registry-v2` and the row-to-CWE mapping of section 9 |
| LGE-R-go | Go: typed panic value for the bit rejection (done, PR #19935) |
| LGE-R-lua, LGE-R-perl, LGE-R-haskell | stable error identity (done: PRs #19936/#19957, #19944, #19942) |
| LGE-GO-SIZEERR | Go: typed `SizeError` for shape/size panics (section 7.1; planned) |
| (unnumbered) | Java/Kotlin `InvalidShapeException` (section 7.1; local branches, unmerged) |
| CV2-S6b/S6c, S7, Go v2 port, S9 | the dependency of section 6 (owned by `portable-warrant.md`) |

## 12. Related specs

[LG01](LG01-logic-gates-contract.md) (the Rust contract and the full
document), [10-logic-gates.md](10-logic-gates.md) (semantics and pedagogy),
[VER04i](VER04i-polyglot-warrant-evidence.md) (polyglot evidence; its section
13.2 is amended by the same change as this document),
[VER04r](VER04r-warrant-corpus-v2.md) (the v2 corpus),
`VER-BACKLOG.d/portable-warrant.md` (CV2 slice status) and
`VER-BACKLOG.d/logic-gates-everywhere.md` (the program queue).

## Errata 2026-11-24 (LGE-S1b)

Recorded when the drift guard (`code/scripts/logic_gates_basic_drift.py`) was
implemented; section 4 above is the intent, these are what the script does.

* **Checks 10-12 are gated by `FIXTURE_REQUIRED`.** The fixture does not exist
  until LGE-S1c. While the module constant `FIXTURE_REQUIRED` is `False` the
  fixture checks are skipped only if the directory is absent AND the version
  control index lists nothing tracked under it; if that query cannot be answered,
  or any error other than "no such file" occurs, the guard fails closed. LGE-S1c
  sets the constant to `True` in the same PR that adds the fixture, after which an
  absent fixture fails.
* **Parsers may not match nothing.** The bindings, the profile, the corpus
  controls and the BUILD pins are each compared with the exact embedded set of 31
  `(op, fault)` pairs (so an empty parse fails), and a pin line that mentions the
  fault variable but is not exactly the Unix or Windows form is an error.
* **All three BUILD variants are parsed** (`BUILD`, `BUILD_linux`, `BUILD_windows`,
  the last in its `set X=..&& set Y=..` form); their basic-op pin sets must be
  identical, and equal the corpus controls once the fixture exists. A corpus
  required claim must be exactly `warrant/logic_gates_basic/exhaustive/<claim>`.
* **Blank and comment lines inside a block are dropped** from the comparison
  (they have no meaning to the grammar); block text is the remaining lines.
* **Example subject = the declared operations the example body calls.** An example
  calling only basic ops is copied; one calling none or only non-basic ops is not;
  one mixing both is an error.
* **Stricter link rule:** any symbolic link anywhere in an input's path is
  refused (the real path must equal the repository path), not only links leading
  outside the root. The opened descriptor must also be the file `lstat` saw.
* **Duplicate block keys** in either document and duplicate corpus controls are
  errors.
* **The `warrantAdopters` row (4.3) is deferred to LGE-S1c**, with the fixture it
  names; the `rust/logic-gates` repository-input row is already in place.

## Errata 2026-11-24 (LGE-LG02-ERRATA)

Every lane's section 7 binding row was re-read from the merged package on
`origin/main`; the lane corpus runners match the exact class, so a stale row
fails the lane. What changed, and why:

* **ruby**: `ArgumentError`/`TypeError` -> `CodingAdventures::LogicGates::InvalidBitError`
  (a subclass of `ArgumentError`; `gates.rb:100,141`, PR #19950).
* **kotlin**: `IllegalArgumentException` -> `com.codingadventures.logicgates.InvalidBitException`
  (`LogicGates.kt:76,501`, PR #19938).
* **java**: `java.lang.IllegalArgumentException` -> `com.codingadventures.logicgates.InvalidBitException`
  (`InvalidBitException.java:14`, PR #19937).
* **csharp / fsharp**: `System.ArgumentOutOfRangeException` -> `CodingAdventures.LogicGates.InvalidBitException`
  (`LogicGates.cs:12`, PR #19939; `LogicGates.fs:16`, PR #19941).
* **elixir**: `ArgumentError` -> `CodingAdventures.LogicGates.InvalidBitError`
  (`invalid_bit_error.ex:21`, PR #19952); the section 9 elixir row's line range is updated.
* **dart**: `ArgumentError` -> `InvalidBitError` (a subclass; `gates.dart:9`, PR #19953).
* **lua**: the `code` is `INVALID_BIT` (not `INVALID_INPUT`; `gates.lua:91`); floats
  such as `1.0` are now rejected, so the stale "fails until the identity
  follow-up lands" text is removed (PRs #19936, #19957).
* **perl**: now a `CodingAdventures::LogicGates::Error` object with code `INVALID_BIT`
  (PR #19944), no longer "to be added".
* **haskell**: `Left String` -> `Left (InvalidBit ...)` of `LogicGateError` (PR #19942).
* **go**: string panic -> typed `*InvalidBitError` (PR #19935); the Go lane is bindable.
* **c, cpp**: rows said "package to be added"; both packages exist (PRs #19926, #19928)
  and the rows now carry source lines. The C++ row is unchanged in value.
* **typescript**: the "library change in progress" for `-0` has merged (PR #19932).
* **python, swift, ocaml, rust**: identities verified unchanged; source lines refreshed
  for python (`gates.py:67-71`).
* **New subsection 7.1** defines the shape/size identity per lane, separately from the
  bad-bit identity, with merged versus planned stated per lane. Runners match the
  bad-bit identity exactly and the shape identity by family.
* The sentence "Today sixteen lanes have a package" is replaced: all eighteen do.

## Errata 2026-11-24 (LGE-S1c)

* **The fixture exists** at `code/specs/fixtures/logic-gates-basic-warrant-v2/` (26 suites, 590,350 cases, 31
  controls, generator 0.6.0), generated without `--bindings`. It has no `consumers.json`, `schema.json` or
  `consumers.schema.json`: the shipped v2 fixtures have none, and section 5's layout list is corrected to
  `corpus.json`, `chunks/`, `corpus.profile`, `cases.tfv`, `README.md`, `.gitattributes`.
* **`FIXTURE_REQUIRED` is `True`**; checks 10 to 12 run. The generator `--check` (BUILD of
  `rust/warrant-corpus-gen`) and the oracle `verify-v2` (BUILD of `python/warrant-oracle`) are wired, with
  `--release` for the generator; the `warrantAdopters` row (section 4.3) exists with an empty `bindings`.
  `max_ir_cost` needed no profile line.
* **Finding: the shift-mask controls name a blind pin.** `shift-mask` on `not_gate` and `nand_not` makes
  `*_rejects_non_bit` blind in the generator's stand-in (a constant expected `Err`) while it is the only claim
  that kills the fault in the Rust harness (`p mod 8` is the identity on 0 and 1; the corpus's required claims
  `*_truth_table` and `nand_not_equals_not` do not fail there). Section 4 check 11 says a pin the corpus calls
  blind fails; the guard now allows exactly these two pins (`KNOWN_STAND_IN_BLIND`) and still fails any other.
  Decision (2026-11-24): the two-pin exception is accepted for now; **LGE-S1d** gives the generator a real
  witness for rejection claims and removes `KNOWN_STAND_IN_BLIND`, and must land before any lane binds
  `shift-mask` literally.
