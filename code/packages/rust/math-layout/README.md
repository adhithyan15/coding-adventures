# math-layout (TEX-1)

Lowers `math_frontend::MathExpr` into a **TeX math list**: a sequence of atoms,
each classified as `Ord`, `Op`, `Bin`, `Rel`, `Open`, `Close`, `Punct` or
`Inner`.

```rust
use math_layout::{lower, Style};

let list = lower(&expr)?;   // fallible: input depth is attacker-controlled
for (atom, space) in list.atoms.iter().zip(list.spacings(Style::Text)) {
    // `space` is the glue that goes BEFORE `atom`.
}
```

## Why the class is the substance

TeX's math spacing is a table indexed by the classes of two adjacent atoms. It
is not a set of aesthetic rules, and it is why `a+b`, `f(x)` and `a=b` space
differently. Classify correctly and the spacing falls out; classify wrongly and
no tuning fixes it, because the difference is structural.

## The demotion rules

A `Bin` is only binary when it has something to bind on **both** sides:

| written | the operator is | because |
|---|---|---|
| `a - x` | `Bin` | it has operands on both sides |
| `-x` | `Ord` | nothing precedes it — it is a sign |
| `a + = b` | `Ord` | a `Rel` follows, so it cannot be binary |

The second rule is the easy one to miss, and it is why the TeXbook marks
`Bin`-followed-by-`Rel` impossible: by the time spacing is chosen, that `Bin`
is an `Ord`.

## The table is extracted, not transcribed

A spacing table is exactly the kind of grid where a transposed row produces
output that looks *almost* right, and a fixture written from the same book by
the same person as the implementation would be wrong in the same way and agree
perfectly.

So `code/scripts/extract_tex_spacing_table.py` asks a real `tex` to typeset all
256 pairs and reads back the glue node it inserted — and TeX names the
parameter it used, so there is no width arithmetic and no threshold of ours
deciding what "thin" means.

It earned its keep on the first run: script-style suppression turns out to be
**per cell**, not per size of space, and the obvious rule is wrong in 30 of the
256 combinations.

```bash
cargo test -p math-layout
python3 code/scripts/extract_tex_spacing_table.py   # needs `tex` on PATH
```

## Untrusted input

`MathExpr` comes from parsing user-supplied LaTeX or AsciiMath, so its nesting
depth is not something the caller controls. Lowering walks that tree
recursively, and before the cap a few hundred kilobytes of `{{{…}}}` aborted
the process with a stack overflow. `lower` returns a `Result` and refuses past
`MAX_NESTING_DEPTH` (256 — real formulas nest to single digits).

## Scope

Pure data transformation — no font metrics, no geometry, no I/O, and no
dependency beyond `math-frontend`. Turning this list into positioned boxes
needs metrics from the `MATH` table and is TEX-3.
