### Added — ALGOL 60 WASM Pipeline
- Advanced PL04 Phase 5 call-by-name lowering with integer array-element
  eval/store thunk descriptors, including repeated re-location of subscripted
  actuals on formal reads and assignments.
- Enabled read-only ALGOL expression thunks to read arrays, covering
  Jensen's-device terms such as `a[i] * i` through the WASM runtime path.
- Enabled read-only ALGOL expression thunks to call integer procedures, with
  nested procedure failures propagated through thunk helper state.
- Added Phase 5 wrap-up coverage and docs for the completed integer by-name
  subset and its remaining full-ALGOL exclusions.
- Added PL04 Phase 6 direct local labels and `goto` support through the
  ALGOL type-checker, IR compiler, and WASM compiler path, with guards for
  nonlocal and Phase 7 designational forms.
- Added PL04 Phase 7a local switch declarations, switch selections, and
  conditional designational `goto` support through type-checking, IR lowering,
  and WASM execution, while keeping nonlocal frame unwinding guarded.
- Added PL04 Phase 7b direct nonlocal block `goto` support with frame/heap
  unwinding inside one lowered function, while keeping procedure-crossing
  jumps and nonlocal designational forms guarded.

