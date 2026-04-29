# beam-bytecode-encoder

Pure encoder: a structured `BEAMModule` description in, raw `.beam`
container bytes out.  This is the inverse of `beam-bytes-decoder` and
the first phase of [BEAM01](../../../specs/BEAM01-twig-on-real-erl.md):
giving the repo the ability to **produce** `.beam` files for real
`erl` to load.

## What this package does

Takes a `BEAMModule` — a frozen dataclass describing atoms, code,
imports, exports, locals, and the label count — and emits a
byte-exact `.beam` IFF container suitable for `erl` to load.

```python
from beam_bytecode_encoder import (
    BEAMModule,
    BEAMInstruction,
    BEAMOperand,
    BEAMExport,
    encode_beam,
)

module = BEAMModule(
    name="hello",
    atoms=("hello", "module_info"),
    instructions=(
        # Emitting a real BEAM module requires func_info + label
        # for every function (see encoder docs).  This README is
        # intentionally short — see tests/ for full examples.
        ...,
    ),
    imports=(),
    exports=(BEAMExport("module_info", arity=0, label=2),),
    locals_=(),
    label_count=2,
)

beam_bytes = encode_beam(module)
# beam_bytes can be written to disk and loaded by `erl -noshell ...`
```

## What this package does NOT do

- **No compiler logic.**  `beam-bytecode-encoder` is a file-format
  writer — it does not lower IR, validate semantics, or check that
  the instruction stream is executable.  Those concerns live in
  `ir-to-beam` and `twig-beam-compiler` (BEAM01 Phases 3 and 4).
- **No literals/funs/lines yet.**  The minimal chunk set is
  `AtU8 + Code + StrT + ImpT + ExpT + LocT`.  `LitT`, `FunT`,
  `Line`, `Attr`, `CInf` are out of scope for v1.

## Where this package fits

```
twig source
  ↓ twig parser / ast_extract
typed AST
  ↓ ir-to-beam            (Phase 3)
BEAMModule (this package's input)
  ↓ encode_beam           (Phase 2 — this package)
.beam bytes
  ↓ erl -noshell -s ...   (real Erlang runtime)
program output
```

The `BEAMModule` shape is deliberately the same as what
`beam-bytes-decoder` would produce from a `.beam` file — so you
can decode a real `erlc`-produced `.beam`, mutate it, and re-encode
it through this package.  That round-trip is what the test suite
exercises.

## Testing

The test suite exercises three layers:

1. **Pure encoder unit tests** — every chunk type encodes to the
   exact bytes the decoder expects.  No `erl` required.
2. **Round-trip via `beam-bytes-decoder`** — encode a `BEAMModule`,
   decode the result, assert structural equality.
3. **Real `erl` load test** (skipped when `erl` not on PATH) —
   write the smallest possible loadable module to disk, ask `erl`
   to load it via `code:load_file/1`, assert success.

## References

- [BEAM01 spec](../../../specs/BEAM01-twig-on-real-erl.md) — the
  three-package roadmap this is the first step of.
- [ECMA-style BEAM file format reference](https://www.erlang.org/doc/apps/erts/beam_file_format)
  — primary source for chunk layouts and compact-term encoding.
