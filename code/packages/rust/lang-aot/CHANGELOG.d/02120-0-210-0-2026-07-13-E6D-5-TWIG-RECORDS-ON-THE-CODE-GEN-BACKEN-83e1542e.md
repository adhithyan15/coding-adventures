## 0.210.0 - 2026-07-13 — E6d-5: Twig records on the code-gen backends

Two matrix cells prove Twig **records** (construct + field access) round-trip:

- `(record Point (x : int) (y : int)) (point-x (Point 42 7))` → 42 (first field)
- `(record Point (x : int) (y : int)) (point-y (Point 7 42))` → 42 (second field)

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. A `(record Name (f : T) …)` erases (in the
Twig frontend, unchanged) to a constructor `Name(…)` that builds a cons chain via
typed `alloc [ref<LispyPair>]` + `field_store`, and accessors `name-f(r)` =
`car(cdr^i(r))` via typed `field_load` — the **E6d-1 heap substrate**, so records
ride the same proven cons/car/cdr path with no new value type and no frontend
change.

Shipping this surfaced and fixed a **latent wasm-runtime bug** (bumped to 0.4.0):
struct field counts were registered by per-function count, which over-counts when
functions share a signature — precisely what a record emits (constructor + N
same-shape accessors + predicate collapse to a few distinct function types). The
`$LispyPair` field count then landed at the wrong (too-high) type index, so the
real `struct.set` type index was a zero-field filler and every record construction
trapped `struct.set: field 0 out of range`. Single-function cons programs and the
list-op helpers were unaffected (their function types were all distinct). Fixed by
indexing on the deduplicated function-type count (`module.types.len()`).
Run-verified locally on WASM (in-process) + real dotnet CLR; native/LLVM/JVM via CI.

