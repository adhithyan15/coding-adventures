# font-parser-python (Rust)

Python C extension wrapping the Rust `font-parser` core. Part of the FNT series.

Exposes `load`, `font_metrics`, `glyph_id`, `glyph_metrics`, and `kerning`
directly to Python with zero copying overhead for the font binary.

## Quick start

```python
import font_parser_native as fp

data = open("Inter-Regular.ttf", "rb").read()
font = fp.load(data)

m = fp.font_metrics(font)
print(m["units_per_em"])   # 2048
print(m["family_name"])    # "Inter"

gid_a = fp.glyph_id(font, ord("A"))
gm    = fp.glyph_metrics(font, gid_a)
print(gm["advance_width"])        # e.g. 1401
print(gm["left_side_bearing"])    # e.g. 7

kern = fp.kerning(font, gid_a, fp.glyph_id(font, ord("V")))
# 0 — Inter v4.0 uses GPOS, not the legacy kern table
```

## How it works

`fp.load()` returns an opaque `PyCapsule` wrapping a `Box<FontFile>` allocated
in Rust. Python's GC calls the capsule destructor (`free_font_file`) when the
handle is collected, which drops the `Box` and frees the memory.

All other functions receive the capsule and call the Rust core library
directly — no serialization, no FFI copies of the font data.

## Building

```bash
cargo build -p font-parser-python --release
# or, to build a wheel:
maturin build --release
```

## Development

```
cargo build --workspace
```

## Testing

There are no Rust unit tests: the shim is only meaningful inside CPython, so the
test imports the built library into a real interpreter and calls it.

```bash
cargo build --release -p font-parser-python --target-dir target
python3 tests/smoke_test.py      # `python` on Windows
```

Each crate builds into its own `target/` (`--target-dir target`) because font-parser-node,
font-parser-python and font-parser-ruby all emit a cdylib named `font_parser_native`.
`build.rs` links `pythonXY.lib` on Windows (located by asking the interpreter for
`sys.base_prefix`; set `PYO3_PYTHON` to pick a specific one).

## License

MIT
