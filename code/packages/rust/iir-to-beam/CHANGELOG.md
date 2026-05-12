# Changelog — iir-to-beam

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.3.0] — 2026-05-12

### Added (LANG35 — Closure Backend Integration)

#### Validator accepts closure opcodes

- `validate_for_beam` now accepts `alloc_closure` and `call_closure` (LANG34
  opcodes) instead of returning `UntypedInstruction` for them.  The validator
  still rejects `call_builtin "make_closure"` / `"apply_closure"` patterns,
  which should be lowered by `iir-builtin-lowering` before reaching a backend.

#### BEAM lowering for `alloc_closure`

- Encodes a closure as a BEAM cons-cell list: `[fn_atom | captures]`.
  - `srcs[0]` must be `Operand::Str(fn_name)` — the callee atom is interned
    into the module atom table at lowering time.
  - `srcs[1..]` are capture variables, each resolved to its x-register.
  - Emits one `put_list` per capture variable (right-fold: innermost cons
    first).  Zero captures → a single `put_list(fn_atom, nil)`.
  - The final cons-cell head is stored in `instr.dest`.

#### BEAM lowering for `call_closure`

- Dispatches a closure via `erlang:apply/3`:
  1. `srcs[0]` is the closure handle (cons-cell list `[fn_atom | caps]`).
  2. Uses `get_list` to split the list into `head` (atom) and `tail` (caps).
  3. Appends user args to caps via `erlang:'++'`/2 (BIF import).
  4. Calls `erlang:apply(Module, Atom, ArgList)` via `erlang:apply/3` (BIF import).
  - Both `erlang:'++'/2` and `erlang:apply/3` are registered as BIF imports.
  - 4 scratch registers are allocated beyond the function's max register.

#### Tests (59–66)

- Tests 59–61: validator acceptance — `alloc_closure` / `call_closure` pass
  validation; `call_builtin "make_closure"` is still rejected.
- Tests 62–64: lowering checks — correct number of `put_list` opcodes for
  0 and 2 captures; `call_closure` emits `get_list` + 2× `call_ext`.
- Test 65 (`test_65_real_erl_arithmetic`): **ignored** — `encode_beam` in
  `ir-to-beam` produces pre-OTP-25 BEAM format rejected by OTP 28 with
  "compiled for an old version".  Requires `Meta` chunk + new `AtU8`
  negative-count encoding to fix.
- Test 66 (`test_66_real_erl_closure_adder`): **ignored** — same root cause
  as test 65.

---

## [0.2.0] — 2026-05-11

### Added (LANG32 — Global Variables and I/O)

#### Global variable support via BEAM process dictionary

- `global_store "x", %v` → `erlang:put(x, %v)` via `gc_bif2`.
  Each global name becomes a BEAM atom constant.
- `global_load "x" → %r` → `erlang:get(x)` via `gc_bif1`, result
  moved to `%r`.
- Atom pre-registration: `erlang:put/2` and `erlang:get/1` BIF
  imports are added to the atom and import tables during module init.

#### I/O support

- `io_out %v` → `erlang:display(%v)` via `gc_bif1`.
  The `erlang:display/1` BIF prints any BEAM term to stdout.

---

## [0.1.0] — 2026-05-11

### Added

- `validate::validate_for_beam(module: &IIRModule) -> Vec<String>` — pre-flight
  validation pass that rejects modules containing BEAM-incompatible instructions
  or types before any lowering starts. Catches:
  - Empty module (no functions)
  - Empty function (function with no instructions)
  - Untyped instructions (`type_hint == "any"` or `"polymorphic"`)
  - Unsupported types (`"str"`, `ref<…>`, float constants)
  - Unsupported opcodes (`call_builtin`, `io_in`, `io_out`, `cast`, memory ops,
    GC ops, `safepoint`)

- `lower::IIRBeamConfig` — lowering configuration, currently just `module_name`.
  Implements `Default` (uses `"iir_module"`) and `new(module_name)`.

- `lower::IIRBeamError` — typed error variants:
  `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`, `UndefinedLabel`,
  `UndefinedVariable`, `InvalidOperand`. Implements `Display` and `std::error::Error`.

- `lower::lower_iir_to_beam(module: &IIRModule, config: &IIRBeamConfig) -> Result<BEAMModule, IIRBeamError>` —
  two-pass lowering algorithm:
  - Pass 1 per function: assign x-registers to params and variable names, scan
    `label` instructions and assign globally-unique BEAM label numbers.
  - Emit `func_info` preamble for each function (`{label,N}`, `{func_info,...}`,
    `{label,N+1}`).
  - Pass 2 per function: translate each `IIRInstr` to BEAM instructions.
  - Build atom table, import table, exports, and final `BEAMModule`.

- Supported IIR opcodes:
  `const` (Int + Bool), `add`, `sub`, `mul`, `div`, `mod`, `neg`,
  `and`, `or`, `xor`, `not`, `shl`, `shr`,
  `cmp_eq`, `cmp_ne`, `cmp_lt`, `cmp_le`, `cmp_gt`, `cmp_ge`,
  `label`, `jmp`, `jmp_if_true`, `jmp_if_false`,
  `ret`, `ret_void`, `call`, `load_reg`, `store_reg`, `type_assert`.

- `codegen::IIRBeamCodeGenerator` — thin adapter that wires `validate_for_beam`
  and `lower_iir_to_beam` behind the `name()` / `validate()` / `generate()` API.

- Re-exported `BEAMModule` and `encode_beam` from `ir-to-beam` for convenience.

- 45 integration tests in `tests/test_backend.rs` covering validation, lowering,
  instruction emission, register allocation, export table, multi-function modules,
  call sequences, and comparison synthesis.
