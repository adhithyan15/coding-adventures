## 0.197.0 - 2026-07-11 (DVAL01-1a: dynamic-value runtime ABI __twig_lispy_* -> __dyn_*)

De-lisp the tagged dynamic-value runtime ABI: every `__twig_lispy_*` C symbol (box_int/unbox_int/cons/car/cdr/pair_p/equal/not/nil/make_symbol/truthy/to_exit_code/tag_*) is renamed to the language-neutral `__dyn_*` (per spec DVAL01). Pure rename -- the 3-bit tag layout, encodings, and runtime behaviour are byte-for-byte unchanged, so any dynamic frontend (not just lisp) can target the same primitives. The GC ABI (`__twig_gc_*`) is untouched.

