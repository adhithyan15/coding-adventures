# python/warrant-corpus

The **Python lane runner** for the language-neutral Warrant evidence corpus
(`warrant-corpus-v1`, [VER04i](../../../specs/VER04i-polyglot-warrant-evidence.md)).

A package's `.warrant` file is compiled, once, by the Rust compiler
(`warrant-grammar`) into a corpus directory under
`code/specs/fixtures/<module>-warrant-v1/`. This package runs that corpus against
a **Python** library and writes the lane's `warrant-lane-evidence-v1` manifest.
Standard library only; it depends on no library it tests.

## Where it fits

```text
code/specs/<pkg>.warrant ──(Rust: parse, check, compile)──► corpus.json + cases.tfv
                                                              │
        python/<pkg>/evidence.bindings  ──►  warrant-corpus (this package)
        (op -> export, exception -> Err)        │  enumerate domains in Python
                                                │  interpret the model IR
                                                │  hash inputs + expected, compare with the corpus
                                                │  call the REAL export, compare
                                                │  break one op at a time (negative controls)
                                                ▼
                              python/<pkg>/evidence/test-manifest.json
```

The corpus does not ship its ~570,000 exhaustive inputs. It ships a descriptor
(`bytes_d1`, `u64_edge`, ...) plus two SHA-256 digests per suite, computed by
Rust over the inputs and over the *model's* answers. This runner regenerates
both streams in Python; a skipped case, a reordered domain or a different model
evaluation changes a digest and fails the suite (`stream-mismatch`).

## Usage

A package keeps a thin entry that fixes its paths (for `ct-compare`:
`tests/warrant_evidence.py`):

```sh
python tests/warrant_evidence.py run --out evidence/test-manifest.json   # regenerate
python tests/warrant_evidence.py check evidence/test-manifest.json       # CI drift check
python tests/warrant_evidence.py sabotage                                # prove it can fail
```

Or directly:

```sh
python -m coding_adventures_warrant_corpus run \
    --repo-root <repo> --corpus <corpus dir> \
    --binding <pkg>/evidence.bindings --package-dir <pkg> --out <manifest>
```

| Command | Meaning |
|---|---|
| `run [--out F]` | examples, exhaustive suites, negative controls; writes the manifest only on a clean full pass |
| `check F` | the same run; exit 1 if the regenerated manifest differs from `F` (ignoring `toolchain.version`) |
| `sabotage` | for every control, a child process with `WARRANT_EVIDENCE_FAULT=<fault>:<op>` must print `not ok` and exit 1 |
| `list-faults` | `<fault>:<op>` for each control |
| `source-digest D` | `warrant-src-v1` digest of a source tree |
| `validate-manifest F [--registry R] [--require-active]` | structural validation against the corpus |

Exit status: `0` all passed, `1` a verdict failed, `2` the run was refused (bad
corpus, bad binding, bad usage). Output is TAP-style: `ok <id>` and
`not ok <id> <failure-id>`, with failures named by suite and case *index* only
(never the input bytes).

Environment: `WARRANT_EVIDENCE_FAULT=<fault>[:<op>]`, `WARRANT_EVIDENCE_FILTER=<substring>`,
`WARRANT_EVIDENCE_OUT=<path>`. A filter without a real fault is refused; a faulted
or filtered run never writes a manifest.

## The binding table

`evidence.bindings` is data, one directive per line:

```text
lane python
package python/ct-compare
module ct_compare
import coding_adventures_ct_compare
source src/coding_adventures_ct_compare
seed 0x5741_5252_414e_5421
values Bool=bool Bytes=bytes Nat=int U64=int
op eq export=ct_eq
op eq_fixed export=ct_eq_fixed width=implicit
op select_bytes export=ct_select_bytes errors=CT_SELECT_LENGTH_MISMATCH:ValueError
op eq_u64 export=ct_eq_u64
```

At start-up the runner checks the table is *total* (every corpus op has a row;
every declared error variant has a mapping and vice versa), that every export
exists, and that the imported module lives under `source` -- so the manifest's
`source_sha256` describes the code that actually ran. An exception that is not *exactly* a mapped class (a subclass does not count) and so matches
no `errors=` row is the verdict `unexpected-error`. Results are checked by exact
type (`type(x) is bool`), because `True == 1` and `bytearray(b"a") == b"a"`.

## The ten faults

`invert`, `ignore-last-byte`, `ignore-high-bit`, `swap-choice`, `missing-error`,
`xor-lsb`, `off-by-one`, `return-first-arg`, `truncate-last`, `drop-to-none`.
Each wraps the *callable the runner holds* (never the library) and perturbs
exactly one op. The corpus lists which `(op, fault)` controls it requires, and
which claims must catch each; a control passes only if a required claim fails
*and* the call reached the real export.

## Honest labels (LP-012)

The corpus calls every enumerated suite `exhaustive`, but exhaustive over `Bytes[347]` means 347 chosen
byte strings. `run --out` therefore also writes `domain-labels.json` beside the manifest, labelling each
suite from its domain descriptors: `full` (the descriptor enumerates the whole type), `bounded-set` (a
fixed strict subset) or `sample` (reserved). Use its `cases_by_label`, not a summed "exhaustive" total:
suite-level totals conflate the kinds until backlog **LP-012** lands.

## Shared conformance fixture (WD-100)

`tests/test_conformance_fixture.py` runs every case of
`code/specs/fixtures/warrant-corpus-conformance-v1/` (VER04i 9.1) through this
lane's own reader, renderer, tagged-value decoder, containment helper and label
digest: 397 cases covering duplicate keys, integers above 2^53, fractions and
exponents, invalid UTF-8, unpaired surrogates, BOMs, nesting depth, `__proto__`
keys, canonical key order and escaping, Windows cross-drive and UNC paths, and
the toolchain-name label digest. The checker that validates the fixture itself
is `python3 code/scripts/warrant_corpus_conformance.py`.

Reader profile in one line: UTF-8 only, no BOM, no duplicate keys (after
unescaping), integers in `+-(2^53 - 1)` only, no unpaired surrogate escapes, at
most 256 nested containers. `containment.relative_inside` compares parsed
paths, so another drive or share is never "inside".

## v2 `ir.core` runner library (VER04r slice S6a)

`coding_adventures_warrant_corpus.v2` is the first part of this lane's **v2** runner. It is the library
for the `ir.core` level, written from VER04r. The package imports nothing from the oracle
(`python/warrant-oracle`), and the conformance kit is the cross-check between the two. A review of the first
version found five pieces that had been adapted from the oracle's kit code (`parse_type`, the value codec,
the chunk-row and chunk-file checks, `fits`, and the domain outcome); those were deleted and rewritten from
sections 2.1 to 2.4, 5.1 to 5.6 and 11 alone, with their own structure. That is what this README claims and no
more: the independence is that of a second reading of one specification, and the repository cannot prove it.

| Module | Implements |
|---|---|
| `wtypes` | section 2.1 type names; `flows` and `unify` |
| `values` | section 2.3 value decoder (every form), section 2.4 stream encoder |
| `arith` | section 3: one wrapping rule; the `Nat`/`Int` window of 2^127 - 1 |
| `ir` | section 4 and Appendix A: static checks, lazy `and`/`or`/`implies`/`if`, call counting, cost |
| `domains` | section 5.1: descriptors, `size()` and `nth()` (`list` is the reserved level `ir.list`, refused) |
| `chunks` | sections 5.2 to 5.5: chunk files, one chunk enumerated alone, the sequential walk |
| `faults` | section 8: the `pre`, `post` and `replace` wrappers |
| `semantics` | the seams where a wrong implementation would differ (below) |
| `kit` | the conformance harness over the v2 kit |

Levels are decided by what an expression reaches ([D64]): a model or op of a level the lane lacks is
classified (`Program.not_live_models`, `not_live_ops`), and only the suites that reach it are `level-missing`. `subject_op`
static widths (`width_params`) and bytes literals typed `Bytes<len>` are implemented (a report's `expect.type` is the
static type, so a bytes literal prints `Bytes<2>`, not `Bytes`). Every consumer of a `types` table enforces
section 2.2's limits (error types of 1 to 64 distinct cases, records of 1 to 32 fields, variants of 2 to 64
constructors of at most 8 fields, no declared name that is a built-in), with the code of the context reading it
(`type` for `v2-value`, `ill-typed` for `v2-ir`, `descriptor` for `v2-domain`, `corpus` for `v2-chunk` and a fault);
an undefined level name in a suite's, fault's or op's `requires` is a corpus error, never `level-missing`; and a
suite row whose `claim` is not one or two legal names is `corpus` (`chunks.chunk_file_name`, section 5.5). It does **not** yet read a v2 corpus, binding, session or manifest (slice S6b), and `python/ct-compare` still
runs `warrant-corpus-v1` (slice S6c).

### Running the v2 conformance kit

```python
from pathlib import Path
from coding_adventures_warrant_corpus.v2 import run_kit

report = run_kit(Path("code/specs/fixtures/warrant-corpus-conformance-v1"))
print("\n".join(report.lines()))      # v2-value: run 128, skipped 0, failed 0 (indexed 128) ...
assert report.ok                      # every category accounted for, nothing failed
```

A lane runs a case if and only if its `requires` is a subset of the levels it implements (`ir.core`) and
skips the rest; per category **run plus skipped equals the index count**. The committed kit has 958 v2 cases: 782 in the six categories this lane
runs (734 from S5b, 48 from S5e), all `ir.core`, 782 run, 0 skipped, 0 failed; and the 176 of `v2-binding` (S5d), a
category this runner does not know, which is indexed, digest-checked and skipped whole (176 skipped) until S6b. Cases with `also_sequential` are also walked sequentially:
chunks `0..=c` from product ordinal 0 must equal the committed chunk file, and chunk `c` enumerated alone by
unranking must equal the walk (about 50 seconds for the 16.7-million-case suite of the kit). A failure line
names the case id and the outcome, never the input bytes.

### The harness can fail (falsification)

`run_kit(..., sem=...)` takes a `Semantics` object. `STANDARD` (the module-level instance of `Semantics`) is the specification; a subclass that
overrides one method is a deliberately wrong implementation. `tests/v2_mutants.py` holds sixteen: no wrap, eager
`and`, first-binder-fastest enumeration (the three VER04r row S6 names), saturating arithmetic, eager `if`, a
signed comparison on the bit pattern, `neg` of the minimum raising, an unchecked `Nat` product, `take` that does
not clamp, a signed stream value that is not sign-extended, a big-endian `u32`, decimal text with a leading zero,
the withdrawn signed `fx_edge` order, `first_admitted` counted on product ordinals, the chunk index missing from
the stream prefix, and `pre` rewrites applied sequentially. `tests/test_v2_falsification.py` requires each to be
reported by case id.

## Bounded work

Before any case runs, a suite's domain product must equal its declared `cases + discarded` and stay
under `MAX_SUITE_PRODUCT`; total work (suites plus control re-runs) is capped by `MAX_TOTAL_WORK`; model
IR has a static cost cap (`MAX_IR_COST`). A corpus that breaks any of these is refused (exit 2).

## Honest limits

A passing run proves the enumerated domain only (for `ct-compare`:
`Bytes[347] x Bytes[347]`, `U64[377] x U64[377]`, ...), never "all inputs". The
Warrant model, the corpus compiler and the binding file are trusted. Nothing here
proves constant-time behaviour of Python code (VER04i section 11, item 5).

## Development

```sh
uv venv --quiet --clear && uv pip install -e ".[dev]" --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m pytest tests/ -v
```
