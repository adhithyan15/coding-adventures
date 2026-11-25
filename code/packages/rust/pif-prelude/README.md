# pif-prelude

The generator of the **PIF prelude** ([VER32](../../../specs/VER32-proof-interchange-format.md)
6.4 and 9.2, backlog item PIF-004b) and the committed file it produces,
[`code/specs/data/pif/prelude.pif`](../../../specs/data/pif/prelude.pif). The prelude is the
first file of every environment chain (prelude, ADL prelude, model, claim): the vocabulary a
gate claim is written in.

```sh
cargo run -q -p pif-prelude > code/specs/data/pif/prelude.pif   # regenerate
cargo test -p pif-prelude                                        # drift fails here
```

## What is in it

* **Inductive types under Lean's names**, with Lean 4 `Init`'s declarations: `Bool`, `Nat`,
  `Int`, `Eq`, `And`, `Or`, `False`, `True`, `Exists`, `Option`, `List`, `Prod`. Universe
  parameters and their names (`u_1` for `Eq`, `u` and `v` elsewhere), parameter counts,
  indices, constructor names and constructor types are Lean's, up to binder names, which are
  ASCII (`alpha` for the Greek letter; R-NAM-3 applies to a reviewed file).
* **`Pif.*`**: `Pif.Not`, `Pif.not`, `Pif.and`, `Pif.or`, `Pif.xor`, `Pif.Nat.add`,
  and `Pif.Result` (`ok`, `err`). The
  functions are written with `Bool.rec` and `Nat.rec` directly: no literals, no features, no
  axioms, no imports. They are **not** `Init`'s definitions (those are elaborator output), so
  the native prelude and `Init` are not interchangeable (VER32 6.4) and the functions carry
  the `Pif.` prefix.

Digest of the committed file (SHA-256, for the gate's allow-list and the
`pif.prelude.faithful` sign-off): `f8df112e5288c86e813d3794b65d3d50554304379071471c2ad7e87c406264c2`.

## Lean as the oracle

`oracle/dump.lean` (run by `oracle/run.sh` with the pinned Lean 4.35.0 toolchain) prints, for
every inductive, what Lean's kernel stores: universe parameters, parameter and index counts,
the type, each constructor's type and the derived recursor's type, in one S-expression per
line; binder names are dropped, binder infos kept. `oracle/expected.sexp` is that output,
committed and reviewed. Lean is not needed to build or test: `tests/prelude.rs` renders the
same S-expression from the environment the trusted reader builds from `prelude.pif` and
compares line by line.

| Type | Params / indices | Constructors | Recursor (kernel-derived) | Result |
|---|---|---|---|---|
| `Bool`, `Nat`, `Int` | 0 / 0 | as Lean | 1 universe parameter (large elimination) | identical |
| `Eq.{u_1}` | 2 / 1 | `refl` | 2 universe parameters | identical |
| `And` | 2 / 0 | `intro` | 1 universe parameter | identical |
| `Or` | 2 / 0 | `inl`, `inr` | no universe parameter (Prop only) | identical |
| `False`, `True` | 0 / 0 | none, `intro` | 1 universe parameter | identical |
| `Exists.{u}` | 2 / 0 | `intro` | 1 universe parameter (Prop only) | identical |
| `Option.{u}`, `List.{u}` | 1 / 0 | `none`/`some`, `nil`/`cons` | 2 universe parameters | identical |
| `Prod.{u,v}` | 2 / 0 | `mk` | 3 universe parameters | identical |
| `Not` (a definition in Lean) | | | | `Pif.Not` has the same type and value |

## Deviations from `Init`, each deliberate

1. **Recursor binder infos.** Lean marks a recursor's parameters, motive and indices implicit;
   the kernel derives them as default. Binder info is display metadata the kernel ignores in
   equality, so the comparison erases it in recursors only. Types and constructors keep their
   binder infos and are compared exactly.
2. **`And` and `Prod` are plain inductives**, not structures: the kernel has no projections
   until slice 3b. Type, constructor and recursor are identical; only `And.left`, `And.right`,
   `Prod.fst` and `Prod.snd` do not exist yet.
3. **`Not` is `Pif.Not`**, a definition with Lean's type and value, as the VER32 PIF-004 row
   says.
4. **`Iff` and `Nonempty` are not in the prelude.** VER32 does not list them there; they are
   pinned for the `prim` roles of 3.5 (a later slice). The oracle already records both, and
   the test asserts exactly these two are absent.
5. **`Pif.Result` is here; `Pif.GateError` is not.** VER32 6.4, the PIF-004 row of section 11
   and the kernel-needs table of 9.2 list `Result` in the PIF prelude, but the chain paragraph
   of 9.2 says the ADL prelude holds it; the coordinator decided it stays here as a generic
   result type. `GateError` is gate-specific and belongs in the gate model file (VER33
   ADL-002), not in a prelude every proof imports. Nothing in VER32 or VER33 needs a `panicked`
   constructor in the prelude, so there is none.
6. **Chosen here, needs review under `pif.prelude.faithful`:** the spec fixes only the name
   `Pif.Result`. The constructor names `ok` and `err` and the layout
   `Result.{u,v} (alpha : Type u) (eps : Type v) : Type (max u v)` (alpha first, as the
   model's `Result Int GateError` reads) are this generator's choice.

## Tests

`tests/prelude.rs`: the committed file equals the generator output; it reads as
`FileClass::Reviewed` with no features, imports, axioms or goals; every declaration has its
expected kind and order; every inductive and recursor equals the oracle; and theorems built
with `pif-writer` and chained by the prelude's digest check through it:
`Pif.not (Pif.not b) = b`, `Pif.xor b b = false`, `Pif.and b true = b`, `Pif.or b false = b`,
`Pif.Nat.add 1 1 = 2`, `Pif.Not False`, a `Pif.Result.ok` value, and a false law the kernel
must refuse.
