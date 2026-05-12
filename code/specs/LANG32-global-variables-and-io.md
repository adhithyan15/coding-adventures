# LANG32 — Global Variables and I/O at the IIR Level

## Why this spec exists

LANG31 wired `call_builtin "+"`, `cons`, `car`, `cdr`, `null?` into the four native
backends (BEAM, WASM, JVM, CLR).  Three equally important builtins were left as
`call_builtin` stubs:

| Builtin      | Emitted by                              | Semantics                              |
|--------------|-----------------------------------------|----------------------------------------|
| `global_set` | top-level `(define name val)` in Twig   | write a module-level variable          |
| `global_get` | top-level name reference in Twig        | read a module-level variable           |
| `print`      | `(print expr)` in Twig                  | write a value to stdout                |

Without these three, every Twig program that uses top-level `define` or prints
output falls back to interpretation — the native backends see unsupported
`call_builtin` instructions and error.

This spec adds them at the **IIR level** so that every language implemented on
top of the LANG pipeline (Twig, NIB, Brainfuck, Prolog, …) gets global variables
and standard output for free.

---

## New IIR opcodes

### `global_load`

```
%dst = global_load("name") : T
```

| Field      | Value |
|------------|-------|
| `op`       | `"global_load"` |
| `dest`     | `Some(register)` |
| `srcs[0]`  | `Operand::Str("name")` — compile-time string name |
| `type_hint`| the type of the variable (`"any"`, `"i64"`, …) |

Semantics: read the current value of the module-level variable named `name`
and place it in `dest`.  Raises an error if the variable has never been set.

Category: **value-producing** (`is_value_producing` returns `true`).

### `global_store`

```
global_store("name", %src) : void
```

| Field      | Value |
|------------|-------|
| `op`       | `"global_store"` |
| `dest`     | `None` |
| `srcs[0]`  | `Operand::Str("name")` — compile-time string name |
| `srcs[1]`  | `Operand::Var(register)` — value to store |
| `type_hint`| `"void"` |

Semantics: write `src` into the module-level variable named `name`.

Category: **side-effecting** (`has_side_effects` returns `true`).

### `io_out` (pre-existing, now lowered)

`io_out` already exists in interpreter-ir.  LANG32 wires it to native backends:

```
io_out(%val) : void
```

Each backend maps this to its native print function:

| Backend | Print call |
|---------|-----------|
| BEAM    | `erlang:display/1` |
| WASM    | imported host fn `$__print_i64` |
| JVM     | `java/lang/System.out.println(J)V` |
| CLR     | `System.Console.WriteLine(int64)` |

---

## New `Operand::Str` variant

`global_load`/`global_store` need a compile-time string operand (the variable
name).  The existing `Operand::Var(String)` is a *register reference* — using
it for string literals is confusing.  LANG32 adds:

```rust
pub enum Operand {
    Var(String),   // register reference
    Int(i64),      // integer literal
    Float(f64),    // floating-point literal
    Bool(bool),    // boolean literal
    Str(String),   // NEW — compile-time string literal (not a register)
}
```

`Str` is distinct from `Var`: a backend that receives `Operand::Str("foo")`
knows it is a literal, not a register to look up.  Future string-value opcodes
(`load_string`, `concat`, …) will reuse this variant.

---

## Lowering rules (in `iir-builtin-lowering`)

The twig-ir-compiler emits:

```
const  %n1 = Operand::Var("x")   -- string literal for global name
call_builtin "global_set", %n1, %val
```

The look-back lowering pass in `global_io.rs`:

1. First pass: build `const_str_map: HashMap<register, literal_text>` for all
   `const` instructions whose `srcs[0]` is `Operand::Var(text)`.
2. Second pass: rewrite `call_builtin "global_set"/%"global_get"` to
   `global_store`/`global_load` using the resolved name from the map.
3. `call_builtin "print"` → `io_out` (one operand, no look-back needed).

Instructions that cannot be resolved (name not a const-string, missing src) are
left as `call_builtin` so the backend validator emits a clear error.

---

## Per-backend implementation

### BEAM

**Globals**: Module-level variables are stored in the BEAM **process dictionary**
via `erlang:put(Key, Value)` and `erlang:get(Key)`.  Each global name becomes
an atom constant.

```
global_store "x", %v  →  atom_x = atom_index("x")
                          move atom_x, Xk
                          call_ext erlang:put/2  (Xk=key, Xk+1=value)
global_load  "x" → %r →  move atom_x, X0
                          call_ext erlang:get/1
                          move X0, %r
```

**I/O**: `io_out %v` → `call_ext erlang:display/1`.

### WASM

**Globals**: WASM has a native **global section**.  Each named global maps to a
`(global i64 (mut i64.const 0))` entry added to `WasmModule::globals`.  The
lowering assigns an index lazily (first encounter → next free slot).

```
global_store "x", %v  →  local.get <slot_of_%v>
                          global.set <idx_of_x>
global_load  "x" → %r →  global.get <idx_of_x>
                          local.set <slot_of_%r>
```

**I/O**: `io_out %v` → `call $__print_i64` (host import added to `WasmModule::imports`).

### JVM

**Globals**: Each named global maps to a `long` static field declared in the
generated class.  Bytecode uses `getstatic`/`putstatic`.

```
global_store "x", %v  →  lload <slot_of_%v>
                          putstatic <field_ref_x>
global_load  "x" → %r →  getstatic <field_ref_x>
                          lstore <slot_of_%r>
```

New bytecodes added to `jvm-class-file`: `getstatic` (0xB2), `putstatic` (0xB3).

**I/O**: `io_out %v` → `getstatic java/lang/System.out`, `lload slot`,
`invokevirtual java/io/PrintStream.println(J)V`.

### CLR

**Globals**: Each named global maps to a `int64` static field declared in the
CIL assembly.  Bytecode uses `ldsfld`/`stsfld` (both already in `ir-to-cil-bytecode`).

```
global_store "x", %v  →  ldloc <slot_of_%v>
                          stsfld <field_token_x>
global_load  "x" → %r →  ldsfld <field_token_x>
                          stloc <slot_of_%r>
```

**I/O**: `io_out %v` → `ldloc slot`, `call System.Console.WriteLine(int64)`.

---

## Module-level state tracking in lowerers

Each backend lowerer grows a `global_slots: HashMap<String, usize>` (or
`HashMap<String, u32>` for WASM/CIL token) that maps each named global to its
backend-specific index/token.  On first encounter of a global name, a new slot
is allocated; subsequent accesses reuse the slot.

---

## Acceptance criteria

1. `(define x 42) (print x)` compiles and executes on all four backends, printing `42`.
2. `(define (inc) (+ x 1)) (define x 0) (set! x (inc)) (print x)` — though `set!`
   is a language-level form, the underlying `global_store` + `global_load` round-trip
   correctly.
3. `io_out` with an `i64` value prints the decimal representation on BEAM, JVM, CLR.
4. `iir-builtin-lowering` tests: ≥ 15 new tests for `global_set`/`global_get`/`print` lowering.
5. `iir-to-beam`/`iir-to-wasm`/`iir-to-jvm-class-file`/`iir-to-cil-bytecode` tests:
   ≥ 5 new tests each for global + io round-trip.

---

## Sister specs

| Spec | Scope |
|------|-------|
| LANG31 | iir-builtin-lowering Phase 1+2; four e2e pipeline crates |
| **LANG32 (this)** | global variables + I/O at IIR level |
| LANG33 | Module system: exports/imports at IIR level; `iir-linker` |
| LANG34 | Closures at IIR level: `alloc_closure`/`call_closure` opcodes |
| LANG35 | Real-VM integration tests (erl/java/dotnet/wasmtime) |
