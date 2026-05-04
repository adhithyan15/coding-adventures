# twig

Twig is a tiny purely-functional Lisp-precursor that runs on
``vm-core``.  Eight forms total, no macros, full closures via a
host-side refcounted heap.

See [TW00 spec](../../../specs/TW00-twig-language.md) for the v1
surface and the multi-spec roadmap (TW01: mark-sweep GC + letrec;
TW02: direct BEAM bytecode emission).

## Why Twig exists

Brainfuck proved out the LANG VM end-to-end on a flat byte tape.
Twig pushes the same infrastructure into territory that Brainfuck
never touches:

* **Heap allocation with lifetime management** — cons cells, symbols,
  and closures live in a host-side ``Heap`` exposed through
  ``call_builtin``.  TW00 ships reference counting; TW01 swaps in
  mark-sweep without changing the language surface.
* **First-class functions with captured environments** — the
  compiler does free-variable analysis at lambda sites and
  routes apply-of-local through ``call_builtin "apply_closure"``.
* **A roadmap to a real Lisp** — Twig's surface is a strict subset
  of Scheme R5RS (minus macros).  Adding ``letrec``, ``set!`` (or
  rejecting it permanently for purely-functional reasons), and
  ``cond`` is straightforward when the GC layer is solid.

## Quick start

```python
from twig import TwigVM

vm = TwigVM()
output, value = vm.run("""
  (define (length xs)
    (if (null? xs) 0 (+ 1 (length (cdr xs)))))
  (length (cons 1 (cons 2 (cons 3 nil))))
""")
print(value)   # 3
```

## Status

- ✅ TW00: lexer / parser / compiler / refcounted heap / TwigVM.
- ⏭ TW01: ``letrec``, mark-sweep GC, cycle handling.
- ⏭ TW02: Twig → BEAM bytecode (direct, no Erlang source).
