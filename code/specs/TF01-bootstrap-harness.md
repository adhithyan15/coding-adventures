# TF01 — The bootstrap harness (L0), TFV, and the run protocol

**Status:** Draft. The Rust lane is implemented first, as step 3 of the VER26
`ct-compare` pilot.

**Builds on:** TF00 (portable test framework), VER26 (pilot: §4, the layering),
VER00 §5 (shuffled order with recorded seeds) and §7.3 (no reproducing material
in public output).

**Revises TF00:** TF00 planned a 19-assertion vocabulary implemented by hand in
every language. VER26 layers it differently. A tiny **bootstrap harness** (this
spec) comes first. **Warrant** (VER04) then compiles a package's examples and
claims into plain tests that need only this harness, and the richer runtime
support is specified once and generated per language. TF00's assertion
vocabulary remains the reference for what Warrant's `example` and `claim`
blocks must be able to express.

---

## 1. Why a bootstrap layer

Every testing tool in the verification program is checked by the layer below
it. The bottom layer has no layer below it, so it must be checkable another
way:

1. **Small enough to verify by reading.** The Rust lane is a few hundred lines
   of plain, literate code with no dependencies.
2. **Pinned by data, not code.** A conformance corpus describes inputs and the
   verdicts expected for them. A harness that passes it agrees with a separate,
   independent reference evaluator. That evaluator is a Python script using
   only the standard library, written separately from any lane.
3. **Proved able to fail.** A harness that never reports failure passes any
   corpus of passing cases. So the corpus includes cases whose verdict must be
   *fail*, and every lane's BUILD runs the harness a second time in **sabotage
   mode** (§6), which must make it report failures.

## 2. TFV v1: the corpus transport

Corpora are written as JSON (`cases.json`), which is pleasant to read and
review. But many languages have no JSON parser in their standard library, and
the bootstrap layer may not depend on third-party code. So each `cases.json`
is compiled by a stdlib-only script into **TFV**, a line format any language
can parse in a few dozen lines. A CI check keeps the two in step.

### 2.1 File shape

- UTF-8, with LF line endings.
- The first line is exactly `tfv 1`.
- Blank lines, and lines whose first character is `#`, are ignored.
- Every other line is one **record**:

  ```
  <id> R<n> K<field> <value> K<field> <value> ...
  ```

  - `<id>` matches `[A-Za-z0-9._/-]+`.
  - `R<n>` gives the number of fields.
  - Field names match `[a-z][a-z0-9_]*`, and no field name repeats within a
    record.

Tokens are separated by **exactly one space**, with no leading or trailing
space.

### 2.2 Values (prefix notation)

| Token | Value | Notes |
|---|---|---|
| `N` | null | |
| `T`, `F` | true, false | |
| `I<int>` | integer | Arbitrary precision. Decimal, optional `-`, no leading zeros, and never `-0` |
| `D<16 hex>` | 64-bit float | The exact IEEE-754 bit pattern, lowercase. `-0` and every NaN payload are exact |
| `X<hex>` | bytes | Lowercase, even length; `X` alone is the empty byte string |
| `S<hex>` | text | The UTF-8 bytes of the string as lowercase hex; `S` alone is the empty string. Hex keeps spaces and newlines out of the token stream |
| `L<n> v1 … vn` | list of `n` values | Ordered |
| `M<n> k1 v1 … kn vn` | map of `n` entries | Keys are values; no two keys may be deep-equal |

Lists and maps nest at most **64** deep: the outermost container is depth 1,
and a container at depth 65 is rejected. The limit bounds a recursive
parser's stack whatever the input, and every lane enforces the same number so
that they accept the same files.

A parser must **reject** any line that breaks these rules. The rejected inputs
include:
- uppercase hex;
- odd-length hex;
- `I-0` or `I007`;
- a count that disagrees with the tokens that follow;
- trailing tokens;
- two spaces in a row;
- a duplicate field;
- a duplicate record id;
- invalid UTF-8 inside `S`;
- a carriage return anywhere in a record line;
- nesting deeper than 64.

The corpus includes such inputs as `tfv_reject` cases (§4).

## 3. The verdict core

The core is a set of **pure functions**. They never throw, never print and
never read configuration. Each returns `Pass` or `Fail(failure_id, detail)`.
The bootstrap layer has only two, because those are all `ct-compare`'s
conformance needs. Warrant generates everything richer.

### 3.1 `deep_equal(actual, expected)`

| Kinds | Equal when |
|---|---|
| null, bool | Same value |
| int | Same mathematical integer |
| f64 | Same bits, after canonicalising NaN: every NaN equals every NaN, and `-0` does **not** equal `+0` |
| bytes, text | Same kind and the same sequence. Bytes never equal text |
| list | Same length and pairwise `deep_equal` |
| map | Same size, and each entry's key has a `deep_equal` key in the other map with a `deep_equal` value |
| different kinds | Never; `I1` does not equal `D3ff0000000000000` |

The failure id is `not-deep-equal`.

### 3.2 `error_is(raised, expected_id)`

`raised` is null (no error was raised) or a text error id.
- Null: `Fail("no-error")`.
- An id other than `expected_id`: `Fail("wrong-error")`.
- Otherwise: `Pass`.

## 4. The self-corpus

`code/specs/fixtures/test-bootstrap-v1/cases.json` → `cases.tfv`. Every record
has an `op` field (text) naming what it tests:

| `op` | Fields | Checks |
|---|---|---|
| `deep_equal` | `actual`, `expected`, `verdict` (`"pass"`/`"fail"`), `failure_id` (fail only) | §3.1 |
| `error_is` | `raised`, `expected`, `verdict`, `failure_id` (fail only) | §3.2 |
| `splitmix64` | `seed`, `outputs` (list of int) | §5.1 |
| `fnv1a64` | `input` (bytes), `hash` (int) | §5.1 |
| `shuffle` | `seed`, `ids` (list of text), `order` (list of text) | §5.2 |
| `sub_seed` | `seed`, `test_id` (text), `sub_seed` (int) | §5.2, rule 4 |
| `tfv_reject` | `text` (text: one complete TFV file) | §2 |

The validator (`code/scripts/bootstrap_harness_conformance.py`) enforces:
- a closed shape for every record;
- unique ids;
- for `deep_equal` and `error_is`, **at least one `pass` and one `fail` case**,
  and every failure id exercised;
- independent re-evaluation of every case by its own reference
  implementation;
- `splitmix64` agreement with the published reference vector;
- that every `tfv_reject` text really is rejected;
- that `cases.tfv` is exactly what `cases.json` generates.

## 5. The run protocol

### 5.1 Primitives

- **SplitMix64.** Each step adds `0x9e3779b97f4a7c15` to the state, wrapping.
  The result is that state run through the standard SplitMix64 finaliser:
  - `z ^= z >> 30`, then `z *= 0xbf58476d1ce4e5b9`;
  - `z ^= z >> 27`, then `z *= 0x94d049bb133111eb`;
  - `z ^= z >> 31`.

  All arithmetic wraps modulo 2^64. With seed 0, the first output is
  `0xe220a8397b1dcdaf`.
- **FNV-1a 64.** Start from `0xcbf29ce484222325`. For each byte, XOR it in,
  then multiply by `0x100000001b3`, wrapping.
- **Bounded draw** `bounded(n)` for 1 ≤ n ≤ 2^32:
  - let `t = (2^64 - n) mod n`;
  - draw `x` until `x ≥ t`;
  - return `x mod n`.

  This is unbiased, and it is the same in every language.

### 5.2 Order, seeds and isolation

1. Tests are registered with ids matching `[A-Za-z0-9._/-]+`. Duplicate ids
   are an error before anything runs.
2. Ids are **sorted by byte order first**. Registration order is never used,
   because JVM and .NET reflection order is unspecified.
3. The sorted list is shuffled by Fisher–Yates driven by
   `SplitMix64(run_seed)`: for `i` from `len − 1` down to 1, swap item `i` with
   item `bounded(i + 1)`.
4. Each test gets its own **sub-seed**: the first output of
   `SplitMix64(run_seed XOR fnv1a64(utf8(id)))`. A test's randomness therefore
   depends only on the run seed and its own id, never on its position.
5. The run seed defaults to 0. A lane accepts `--seed <decimal u64>`
   (ASCII digits only).

A panic or exception escaping a test is that test's failure, with failure id
`panic`. It does not stop the run. The panic's own message is **not** copied
into TAP, because it may quote the inputs that caused it; a lane may leave it
on stderr.

An invalid or duplicate test id is reported as a single TAP
`Bail out! <reason>` line before any test runs, and the run fails.

### 5.3 Output: a TAP v14 profile

```
TAP version 14
# tf-lane: rust
# tf-seed: 0
1..3
ok 1 - ct-compare/eq-empty
not ok 2 - ct-compare/eq-last-byte
  ---
  failure_id: not-deep-equal
  message: "expected T, got F"
  ...
ok 3 - ct-compare/eq-u64-zero
```

- The YAML block keys are pinned: `failure_id`, `message`. `property:` and
  `fuzz:` are reserved for VER05 and VER06. Both values are YAML
  double-quoted scalars, with every control and line-breaking character
  escaped, so a message cannot end the block or forge a TAP line.
- Failure ids form a closed vocabulary. L0 defines:

  | Id | Meaning |
  |---|---|
  | `not-deep-equal`, `no-error`, `wrong-error` | from the verdict core (§3) |
  | `panic` | a panic or exception escaped the test |
  | `corpus-mismatch` | the self-corpus expected a different answer |
  | `corpus-malformed` | a self-corpus record lacks the fields its `op` needs |

- The process exits **0 when every test passed, 1 otherwise** (including a
  bail-out, or standard output closing before the report is written), and
  **2 for a command line it does not understand**. Lanes whose build tool
  forwards its own flags to every test binary may ignore those flags by
  name; the Rust lane ignores Cargo's `--nocapture`.
- The seed printed is the *run* seed, which is harmless.
- Property and fuzz failures must never print their reproducing seed or input
  in public CI output (VER00 §7.3). That rule belongs to the engines that
  produce such failures, and this layer gives them nowhere to put it.

## 6. Sabotage mode

Run with `--sabotage always-pass`, the corpus checker treats **every** verdict
from the core as `Pass`. The corpus's `fail` cases then disagree with their
expected verdicts, so a working harness reports them as `not ok` and exits 1.
Each lane's BUILD runs:

1. the self-corpus normally, which must exit 0; and
2. the self-corpus in sabotage mode, whose output must contain at least one
   line starting with `not ok `.

The second check is done **outside** the harness, with `grep` or `findstr` on
its output. A harness that could not report failure would fail the BUILD
there. The check never relies on the harness grading itself.

## 7. What L0 deliberately is not

It has:
- no assertion beyond `deep_equal` and `error_is`;
- no fixtures, hooks or parallelism;
- no coverage;
- no file access: corpora are embedded at compile time, so the crate needs no
  capabilities.

All of that belongs to the layers above, which this layer is used to check.
