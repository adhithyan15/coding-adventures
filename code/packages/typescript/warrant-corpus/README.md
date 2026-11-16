# typescript/warrant-corpus

The **TypeScript lane runner** for the language-neutral Warrant evidence corpus
(`warrant-corpus-v1`, [VER04i](../../../specs/VER04i-polyglot-warrant-evidence.md)).
It is the second non-Rust lane, written to match the
[Python runner](../../python/warrant-corpus) check for check.

A package's `.warrant` file is compiled, once, by the Rust compiler
(`warrant-grammar`) into a corpus directory under
`code/specs/fixtures/<module>-warrant-v1/`. This package runs that corpus against
a **TypeScript** library and writes the lane's `warrant-lane-evidence-v1`
manifest. It has no runtime dependency (Node's built-in `crypto` does the
hashing) and depends on no library it tests.

## Where it fits

```text
code/specs/<pkg>.warrant ──(Rust: parse, check, compile)──► corpus.json + cases.tfv
                                                              │
      typescript/<pkg>/evidence.bindings ──►  warrant-corpus (this package)
      (op -> export, error class -> Err)          │  enumerate domains in TypeScript
                                                  │  interpret the model IR (no eval)
                                                  │  hash inputs + expected, compare with the corpus
                                                  │  call the REAL export, compare
                                                  │  break one op at a time (negative controls)
                                                  ▼
                            typescript/<pkg>/evidence/test-manifest.json
```

The corpus does not ship its ~570,000 exhaustive inputs. It ships a descriptor
(`bytes_d1`, `u64_edge`, ...) plus two SHA-256 digests per suite, computed by
Rust over the inputs and over the *model's* answers. This runner regenerates
both streams in TypeScript; a skipped case, a reordered domain or a different
model evaluation changes a digest and fails the suite (`stream-mismatch`).

## Running it

The runner is TypeScript executed directly by Node's type stripping, so the code
that runs is the code that is digested (`runner.src_sha256`). A package keeps a
thin entry that fixes its paths (for `ct-compare`: `tests/warrant-evidence.ts`):

```sh
node --experimental-strip-types tests/warrant-evidence.ts run --out evidence/test-manifest.json   # regenerate
node --experimental-strip-types tests/warrant-evidence.ts check evidence/test-manifest.json       # CI drift check
node --experimental-strip-types tests/warrant-evidence.ts sabotage                                # prove it can fail
```

Or directly:

```sh
node --experimental-strip-types src/bin.ts run \
    --repo-root <repo> --corpus <corpus dir> \
    --binding <pkg>/evidence.bindings --package-dir <pkg> --out <manifest>
```

| Command | Meaning |
|---|---|
| `run [--out F]` | examples, exhaustive suites, negative controls; writes the manifest and `domain-labels.json` only on a clean full pass |
| `check F` | the same run; exit 1 if the regenerated manifest or labels differ from the committed files (ignoring `toolchain.version`) |
| `sabotage` | for every control, a child process with `WARRANT_EVIDENCE_FAULT=<fault>:<op>` must print `not ok` and exit 1 |
| `list-faults` | `<fault>:<op>` for each control |
| `source-digest D` | `warrant-src-v1` digest of a source tree |
| `validate-manifest F [--registry R] [--require-active]` | structural validation against the corpus (it accepts a manifest from any lane) |

Exit status: `0` all passed, `1` a verdict failed, `2` the run was refused (bad
corpus, bad binding, bad usage, or a bug in the runner). Output is TAP-style:
`ok <id>` and `not ok <id> <failure-id>`, with failures named by suite and case
*index* only (never the input bytes).

Environment: `WARRANT_EVIDENCE_FAULT=<fault>[:<op>]`, `WARRANT_EVIDENCE_FILTER=<substring>`,
`WARRANT_EVIDENCE_OUT=<path>`. A filter without a real fault is refused; a faulted
or filtered run never writes a manifest.

## The binding table

`evidence.bindings` is data, one directive per line (VER04i section 5.2):

```text
lane typescript
package typescript/ct-compare
module ct_compare
import src/index.ts
source src
seed 0x5741_5252_414e_5421
values Bool=boolean Bytes=Uint8Array Nat=bigint U64=bigint
op eq export=ctEq
op eq_fixed export=ctEqFixed width=implicit
op select_bytes export=ctSelectBytes errors=CT_SELECT_LENGTH_MISMATCH:RangeError
op eq_u64 export=ctEqU64
```

At start-up the runner checks the table is *total* (every corpus op has a row;
every declared error variant has a mapping and vice versa), that every export
exists, and that the imported module's real path lives under `source`, so the
manifest's `source_sha256` describes the code that actually ran. `import` is a
path, never a bare package name, so a copy under `node_modules` cannot stand in.

What differs from Python, and why:

- **`U64` and `Nat` are `bigint`.** A `number` is a double: `2^64-1` and
  `2^64-2` would round to the same value. The binding refuses `U64=number`.
- **Exact result types.** A `U64` result that is the `number` `1`, a `Bytes`
  result that is a Node `Buffer` or any subclass, a detached array and a
  promise all fail with `type-mismatch`.
- **Errors map by exact prototype.** A thrown value is the declared `Err` only if
  its prototype *is* `RangeError.prototype` (a closed list of built-in classes;
  `Error` itself is refused). A subclass, a thrown string and `null` are
  `unexpected-error`.
- **Bytes are mutable.** Every `Bytes` argument is a fresh copy, so a library that
  writes to its input cannot corrupt the enumerated domain, and a `fresh` result
  may not share an `ArrayBuffer` with an argument (rule 4).
- **JSON is read by the runner's own parser**, because `JSON.parse` keeps the last
  of two equal keys and rounds integers above 2^53 silently. Corpus text must be
  well-formed Unicode: invalid UTF-8 and unpaired surrogate escapes are refused
  (the Python lane still accepts a lone-surrogate escape).

## The model IR runs without `eval`

Claim bodies are tiny expression trees. They are compiled once, before any case
runs, into closures over an array of variable slots. Nothing turns corpus text
into code, so a hostile corpus can only select among the node kinds
`src/ir.ts` implements; an unknown node is a refusal, never a skipped case.

## The ten faults

`invert`, `ignore-last-byte`, `ignore-high-bit`, `swap-choice`, `missing-error`,
`xor-lsb`, `off-by-one`, `return-first-arg`, `truncate-last`, `drop-to-none`.
Each wraps the *callable the runner holds* (never the library) and perturbs
exactly one op. The corpus lists which `(op, fault)` controls it requires, and
which claims must catch each; a control passes only if a required claim fails
*and* the call reached the real export.

## Honest labels (LP-012)

The corpus calls every enumerated suite `exhaustive`, but exhaustive over
`Bytes[347]` means 347 chosen byte strings. `run --out` therefore also writes
`domain-labels.json` beside the manifest, labelling each suite from its domain
descriptors: `full` (the descriptor enumerates the whole type), `bounded-set` (a
fixed strict subset) or `sample` (reserved). Use its `cases_by_label`, not a
summed "exhaustive" total: suite-level totals conflate the kinds until backlog
**LP-012** lands.

## Bounded work

Before any case runs, a suite's domain product must equal its declared
`cases + discarded`, a suite must admit at least one case, and the product stays
under `maxSuiteProduct` (1,000,000). Total work (suites plus control re-runs) is
capped by `maxTotalWork` (8,000,000), model IR has a static cost cap
(`MAX_IR_COST`, 2,000) and a depth cap, and arithmetic operands are range-checked
before the operation. A corpus that breaks any of these is refused (exit 2).

## Honest limits

A passing run proves the enumerated domain only (for `ct-compare`:
`Bytes[347] x Bytes[347]`, `U64[377] x U64[377]`, ...), never "all inputs". The
Warrant model, the corpus compiler and the binding file are trusted. Nothing here
proves constant-time behaviour of JavaScript code, which a JIT is free to
reshape (VER04i section 11, item 5). The runner is not yet run under the VER08b
sandbox (backlog CAP-RT-1xx).

## Development

```sh
npm ci
npm run lint            # tsc --noEmit (strict, erasable syntax only)
npx vitest run --coverage
```

The suite includes one run of the whole real corpus (572,109 cases); everything
else uses a small slice of it so the plumbing tests stay fast. Test fixtures are
copied as bytes, never read and rewritten as text.
