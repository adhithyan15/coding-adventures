# AOT00 — Twig native-AOT GC coverage (capstone: COMPLETE)

> Status: **complete.** Every heap object the Twig native-AOT path allocates is under the gc-core
> collector (`FlatHeap`, via the `gc-core-capi` C ABI) — traced, reclaimed, and, where its layout is
> known, **relocated** under compaction. This document is the single per-heap-type accounting that
> closes the user directive *"get Twig working end to end; make sure all the language features Twig
> supports have a native GC that can be included in the AOT path."*

## The directive

The gc-core engine itself was completed over the AOT00-T1…T5 arc: mark-sweep → interior-precise →
generational → **precise roots** (all three native targets) → **compacting** (moving) →
**incremental**, plus T5 variable-length reference arrays. This capstone is about the *frontend*:
proving each Twig heap **feature** actually routes its allocations through that collector on the
native (aarch64 / x86_64 → Mach-O/ELF/PE → system linker) path, rather than leaking or pinning.

## Per-heap-type coverage

| Twig heap type | native allocation | GC status | proof |
|---|---|---|---|
| **cons / pairs / lists** | `__dyn_cons` → `__gc_alloc_kind({0,8})` | precise + **movable** | `end_to_end_gc_compacting_relocates_and_preserves` |
| **closures** | lowered to a `__dyn_cons` cons chain `(box(idx) . caps…)` (E6d-7a) + a synthesized `__dyn_call_closure` dispatcher | precise + **movable** | `end_to_end_closure_captured_env_survives_collect` (#9274); `closures_run_on_native` |
| **records / unions** | generic `alloc` → `__twig_gc_alloc_pair` → `__gc_alloc_kind({0,8})` | precise + **movable** | `end_to_end_gc_record_field_traced_and_relocated`; `records_run_on_native` |
| **strings** | `__twig_alloc_bytes` → `__gc_register_kind(NULL,0)` (no-reference blob) + `__gc_alloc_kind` | GC-managed **leaf** (traced, reclaimed; movable by pin-when-unsure) | `end_to_end_gc_manages_runtime_strings` (#9218) |
| **symbols** | *none* — `intern_symbols` rewrites each symbol literal to the tagged immediate `(id<<32) \| 0b010` at compile time | **not a heap object** (an immediate, like an int) | `symbol_intern` unit tests |
| **ints / nil / bools** | *none* — immediate tagged words | **not a heap object** | — |

So the native-AOT heap surface partitions cleanly into: **precise + movable** (cons/lists,
closures, records/unions), a **GC-managed leaf** (strings — an opaque byte blob has no interior
references to trace, so a no-reference kind is both correct and the most precise choice), and
**immediates** (symbols, ints, nil, bools — never on the heap). Nothing leaks; nothing is
conservatively pinned as a permanent compromise.

## How records became movable (the last rung)

A Twig record `(record Point (x : int) (y : int))` erases (via `twig-ir-compiler`'s
`emit_record_def`) to the generic `alloc` + `field_store` ops — a two-word `ref<LispyPair>` cell
whose constructor parameters are typed `any`, so **its two stored fields are always boxed** (a
tagged immediate or a heap reference — never a raw look-alike integer). That makes the precise
`{0,8}` pair kind (both words are reference slots) **sound** for a record, identical to a cons cell.

The native backends now lower that `alloc` to `__twig_gc_alloc_pair` (gc-core-capi 0.22.0), which
allocates under the movable `{0,8}` kind, instead of:

- aarch64's kind-0 `__twig_gc_alloc` — conservative, correct but **pinned**; and
- x86_64's `__twig_alloc_bytes` — which, since it was routed through gc-core as a **no-reference
  blob** kind for strings (twig-aot 0.48.0), left a record's reference fields **untraced**: a child
  reachable only through a record field could be reclaimed under a live record (a use-after-free).
  Routing to the traced `{0,8}` kind **fixes that latent bug** as a side effect.

The `end_to_end_gc_record_field_traced_and_relocated` differential exercises the whole chain: a
record holds a heap child in field 0; a **compacting** collect relocates the record *and* the child
and rewrites field 0 (a **raw**, untagged pointer — a distinct root-fixup path from the tagged
cons-cell tests); reading the child back through the record still yields 42.

**Unions share the cell but not the boxed-fields property — and are still sound.** `emit_union_def`
stores a synthesized integer **discriminant** in word 0 of the same `{0,8}` cell. That word is not a
boxed `any`, so the boxed-fields argument does not cover it — but gc-core is **provenance-filtered**:
`fixup_ref_fields` relocates a ref slot only when its value is a key in the compaction forwarding map
(the old address of a real moved block), and the mark phase follows a slot only when `find_header`
resolves it to a live block. A small discriminant is neither, so it is never followed or rewritten,
regardless of its low bits. `end_to_end_gc_raw_discriminant_word_not_relocated` proves it: a cell
whose word 0 is the heap-tag-looking `0b10111` (23) and whose word 1 is a live heap child relocates
under compaction, and word 0 reads back as 23 verbatim. Provenance filtering — not tagging — is the
guarantee for non-`any` words in a `{0,8}` cell.

## Related specs

- `AOT00-T5-variable-length-ref-arrays.md` — the `register_ref_array_kind` / `__gc_alloc_kind`
  machinery reused here.
- `AOT00-T6-native-closures.md` — closures were found already native + GC-managed (cons-chain
  lowering); its env-pointer codegen is an unscheduled optimization, not a gap.
