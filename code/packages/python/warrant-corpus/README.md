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
`source_sha256` describes the code that actually ran. An exception that matches
no `errors=` row is the verdict `unexpected-error`. Results are checked by exact
type (`type(x) is bool`), because `True == 1` and `bytearray(b"a") == b"a"`.

## The ten faults

`invert`, `ignore-last-byte`, `ignore-high-bit`, `swap-choice`, `missing-error`,
`xor-lsb`, `off-by-one`, `return-first-arg`, `truncate-last`, `drop-to-none`.
Each wraps the *callable the runner holds* (never the library) and perturbs
exactly one op. The corpus lists which `(op, fault)` controls it requires, and
which claims must catch each; a control passes only if a required claim fails
*and* the call reached the real export.

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
