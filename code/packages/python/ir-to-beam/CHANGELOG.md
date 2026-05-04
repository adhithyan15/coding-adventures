# Changelog — ir-to-beam

## 0.4.0 — 2026-04-30 — TW03 Phase 3d (BEAM heap primitives)

Implements the BEAM-side lowering for the eight TW03 Phase 3a heap
opcodes.  An IR program using any heap opcode now compiles to a
real BEAM module that runs on stock `erl` and produces correct
list-walking output.

BEAM's the easiest of the three native backends here because cons
cells and atoms are first-class BEAM terms with native opcodes.
No "runtime classes" needed — we just emit the right opcode for
each IR op.

### Added — eight opcode lowerings

| IR op | BEAM emission |
|---|---|
| `MAKE_CONS dst, head, tail` | `test_heap 2 0; put_list y{head}, y{tail}, y{dst}` |
| `CAR dst, src` | `get_hd y{src}, y{dst}` |
| `CDR dst, src` | `get_tl y{src}, y{dst}` |
| `IS_NULL dst, src` | `is_nil F y{src}; move {integer,1}, y{dst}; jump END; F: move {integer,0}, y{dst}; END:` |
| `IS_PAIR dst, src` | `is_nonempty_list F y{src}; …` (same true/false dance) |
| `IS_SYMBOL dst, src` | `is_atom F y{src}; …` |
| `MAKE_SYMBOL dst, name_label` | `move {atom, idx}, y{dst}` (atom interned via `builder.atoms.add`) |
| `LOAD_NIL dst` | `move {atom, 0}, y{dst}` (atom 0 = nil) |

### Added — three new BEAM opcode constants

`_OP_IS_ATOM` (48), `_OP_IS_NIL` (52), `_OP_IS_NONEMPTY_LIST` (56).
Names line up with `beam_opcode_metadata.catalog`.

### Test additions

- 8 new structural unit tests covering each opcode's lowering
  shape (asserts on emitted opcode bytes + atom-table contents).
- 2 new arity-validation tests (MAKE_CONS / LOAD_NIL with wrong
  operand counts get a clear diagnostic).
- 2 new real-`erl` end-to-end tests:
  - `test_heap_list_of_ints_length_returns_3` — builds
    `[1, 2, 3]` via MAKE_CONS / LOAD_NIL, walks via CDR /
    IS_NULL, returns the integer 3.
  - `test_heap_make_symbol_returns_atom` — `MAKE_SYMBOL` with
    name `foo` returns the atom `foo` from real `erl`.
- All 41 BEAM tests pass; coverage 88%.

### Limitations

None — BEAM atoms are global per VM (so symbol interning is
free), nil is a first-class BEAM term, and cons cells get
reclaimed by BEAM's own GC.  Phase 4 (GC) is automatic on BEAM.

## 0.3.0 — 2026-04-29 — TW03 Phase 2 (BEAM closures)

### Added — ``MAKE_CLOSURE`` and ``APPLY_CLOSURE`` lowering

- ``MAKE_CLOSURE dst, fn_label, num_captured, capt0, ..., captN-1``
  lowers to a chain of ``put_list`` opcodes that build the closure
  value as the cons cell ``[FnAtom | [capt0, capt1, ..., captN-1]]``
  on the heap (preceded by a ``test_heap`` reservation).
- ``APPLY_CLOSURE dst, closure, num_args, arg0, ..., argM-1``
  lowers to: build args list, ``get_tl`` for captures,
  ``erlang:'++'/2`` to glue them, ``get_hd`` for the function
  atom, then ``erlang:apply/3`` for the dynamic dispatch.
- ``BEAMBackendConfig.closure_free_var_counts`` — declares which
  callable regions are lifted lambda bodies and how many captured
  free variables each takes.  The backend widens
  ``arity_overrides[name]`` to ``num_free + explicit`` so apply/3
  can find the lifted lambda by its full arity.
- Lifted lambdas are now exported (apply/3 needs to look them up
  by atom name in the export table).

### Why we don't use ``make_fun2`` / ``make_fun3``

Real ``erlc`` emits ``make_fun3`` (opcode 171), which uses the
z-tagged extended-list compact-term encoding our encoder doesn't
yet support.  The older ``make_fun2`` (opcode 103) is rejected by
Erlang/OTP 28 with "please re-compile with an OTP 28 compiler".
Encoding closures as plain cons-cell lists + ``erlang:apply/3``
side-steps both problems and uses only standard, well-supported
opcodes.  The runtime cost is one ``apply`` indirection per
invocation; the engineering benefit is a closure pipeline that
loads cleanly under modern OTP.

### Added tests

- ``test_closure_make_adder_returns_42`` —
  ``((make-adder 7) 35) → 42`` end-to-end on real ``erl``,
  exercising MAKE_CLOSURE + APPLY_CLOSURE in the same module.

## 0.2.0 — 2026-04-29 — TW03 Phase 1 (BEAM)

### Added — branching, comparison, and recursion

- ``BRANCH_Z`` / ``BRANCH_NZ`` lowering via BEAM's ``is_ne_exact`` /
  ``is_eq_exact`` (whose "fall through if condition holds, jump
  otherwise" semantics flip to direct BRANCH semantics with
  appropriate operand choice).
- ``JUMP`` lowering to BEAM's ``jump`` opcode.
- ``CMP_EQ`` / ``CMP_LT`` / ``CMP_GT`` lowering — each becomes a
  5-instruction if-then-else pattern (``is_X`` + move 1 + jump +
  label + move 0 + label).  ``CMP_GT`` uses ``is_lt`` with swapped
  operands since BEAM has no ``is_gt``.
- ``ADD_IMM`` lowering — single ``move`` for ``imm == 0`` (the
  Twig MOV idiom), ``gc_bif2`` otherwise.
- Internal LABEL handling: ``_split_callable_regions`` now
  distinguishes callable regions (CALL targets + entry) from
  internal labels (``_else_*``, ``_endif_*``); internal labels
  stay in the body and emit ``label N`` opcodes.

### Changed — y-register frame for recursion support

- Function bodies now use BEAM **y-registers** (callee-saves stack)
  for all IR registers, with **x-registers** used only at CALL
  boundaries.  This is necessary for recursive functions because
  BEAM x-registers are caller-saves (clobbered across calls).
- Each function emits ``allocate K, arity`` at entry (K = max IR
  register index used) and ``deallocate K`` before ``return``.
- At function entry, args are copied from BEAM ``x0..x{arity-1}``
  into the IR's param-slot registers ``y2..y{arity+1}``.
- At each CALL site, args are copied from ``y{2+i}`` to ``x{i}``,
  then ``call N, label``, then the result is copied from ``x0``
  back into the IR's HALT-result register ``y1``.

### Added tests

- ``test_recursive_factorial_returns_120`` — hand-built IR
  exercising ``allocate`` + ``call`` + ``deallocate`` +
  ``BRANCH_Z`` + ``CMP_EQ`` + arithmetic across recursive call
  boundaries.  Confirms ``fact(5) → 120`` on real ``erl``.
- Tests for each new IR-op lowering.

### Migration notes for existing callers

The IR convention is now **r1 holds the function's return value**
(matches twig-jvm-compiler / twig-clr-compiler).  Programs that
used ``r0`` as the result must update.

## 0.1.0 — 2026-04-29

### Added — BEAM01 Phase 3: compiler-ir → BEAMModule lowering

- ``BEAMBackendConfig`` — configures module name + which IR
  callable region is the entry function.
- ``BEAMBackendError`` — raised on unsupported IR ops or
  structurally invalid programs.
- ``lower_ir_to_beam(ir, config) -> BEAMModule`` — the entry
  point.
- IR-op → BEAM-op coverage (v1):
  - ``LABEL``, ``LOAD_IMM``, ``ADD``, ``SUB``, ``MUL``, ``DIV``,
    ``CALL``, ``RET``.
- Auto-injected ``module_info/0`` and ``module_info/1`` exports
  delegating to ``erlang:get_module_info/{1,2}``.
- Tests:
  - Round-trip parity via ``beam-bytes-decoder`` — every emitted
    module decodes cleanly with matching atom / export / import
    tables.
  - Real-``erl`` smoke tests (skipped without ``erl`` on PATH):
    a synthesised ``add(17, 25)`` returns 42, a synthesised
    ``identity(99)`` returns 99.

### Out of scope (future iterations)

- ``BRANCH_Z`` / ``BRANCH_NZ`` / ``JUMP`` — control flow needs
  live-register tracking.
- ``SYSCALL`` — output / I/O.
- Memory ops (``LOAD_BYTE`` / ``STORE_BYTE``) — Twig doesn't use
  them.
