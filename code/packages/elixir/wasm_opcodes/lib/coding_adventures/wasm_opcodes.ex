defmodule CodingAdventures.WasmOpcodes do
  @moduledoc """
  Complete WASM 1.0 opcode lookup table with metadata for every instruction.

  This module is part of the coding-adventures monorepo — a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## What is a WASM opcode?

  A WebAssembly binary is a sequence of *sections*. The code section holds
  function bodies, each of which is a flat byte sequence of *instructions*.
  The first byte of every instruction is its **opcode** — a value 0x00–0xBF
  in WASM 1.0 (multi-byte opcodes via 0xFC prefix exist in later proposals,
  but are out of scope here).

  ```
  Function body byte stream example:

    0x20 0x00      ← local.get  $local_0
    0x20 0x01      ← local.get  $local_1
    0x6A           ← i32.add
    0x0F           ← return
    0x0B           ← end
  ```

  ## The operand stack

  WASM is a **stack machine**. Instructions consume values from a virtual
  operand stack (`stack_pop`) and push results back onto it (`stack_push`).

  ```
  Before i32.add:   [..., 3, 7]
  After  i32.add:   [..., 10]    ← popped 2, pushed 1
  ```

  The `:stack_pop` and `:stack_push` fields encode this for each instruction.
  For control instructions (block/loop/if/call) these are 0/0 because the
  actual arity depends on the block type or function signature at runtime.

  ## Immediates

  Many instructions carry **immediate** arguments encoded directly in the
  byte stream right after the opcode byte. For example:

  ```
  Instruction          Immediates
  ─────────────────────────────────────────────
  local.get $0         localidx  (LEB128 u32)
  i32.const 42         i32       (signed LEB128)
  i32.load offset=8    memarg    (align:u32, offset:u32)
  br_table [0,1,2] 3   vec_labelidx (count + labels + default)
  ```

  The `:immediates` field is a list of string names describing what follows
  the opcode byte in the binary.

  ## Structured control flow

  Unlike x86 machine code, WASM has *no* unstructured jumps. All branches
  target enclosing blocks identified by a label depth index:

  ```
  block $outer        ;; depth 1 from inside
    block $inner      ;; depth 0 from inside
      br 0            ;; branch to $inner's end
      br 1            ;; branch to $outer's end
    end
  end
  ```

  `loop` differs from `block` in that `br` targeting a loop goes to its
  *start* (backward), while `br` targeting a block goes past its *end*
  (forward). This prevents arbitrary control flow while enabling all loops.

  ## Complete WASM 1.0 opcode table (172 entries)

  ```
  ┌─────────┬─────────────────────────┬──────────────┬────────────────────────┬─────┬──────┐
  │ Opcode  │ Name                    │ Category     │ Immediates             │ Pop │ Push │
  ├─────────┼─────────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
  │ Control instructions                                                                    │
  │ 0x00    │ unreachable             │ control      │ —                      │  0  │  0   │
  │ 0x01    │ nop                     │ control      │ —                      │  0  │  0   │
  │ 0x02    │ block                   │ control      │ blocktype              │  0  │  0   │
  │ 0x03    │ loop                    │ control      │ blocktype              │  0  │  0   │
  │ 0x04    │ if                      │ control      │ blocktype              │  1  │  0   │
  │ 0x05    │ else                    │ control      │ —                      │  0  │  0   │
  │ 0x0B    │ end                     │ control      │ —                      │  0  │  0   │
  │ 0x0C    │ br                      │ control      │ labelidx               │  0  │  0   │
  │ 0x0D    │ br_if                   │ control      │ labelidx               │  1  │  0   │
  │ 0x0E    │ br_table                │ control      │ vec_labelidx           │  1  │  0   │
  │ 0x0F    │ return                  │ control      │ —                      │  0  │  0   │
  │ 0x10    │ call                    │ control      │ funcidx                │  0  │  0   │
  │ 0x11    │ call_indirect           │ control      │ typeidx, tableidx      │  1  │  0   │
  ├─────────┼─────────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
  │ Parametric instructions                                                                 │
  │ 0x1A    │ drop                    │ parametric   │ —                      │  1  │  0   │
  │ 0x1B    │ select                  │ parametric   │ —                      │  3  │  1   │
  ├─────────┼─────────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
  │ Variable instructions                                                                   │
  │ 0x20    │ local.get               │ variable     │ localidx               │  0  │  1   │
  │ 0x21    │ local.set               │ variable     │ localidx               │  1  │  0   │
  │ 0x22    │ local.tee               │ variable     │ localidx               │  1  │  1   │
  │ 0x23    │ global.get              │ variable     │ globalidx              │  0  │  1   │
  │ 0x24    │ global.set              │ variable     │ globalidx              │  1  │  0   │
  ├─────────┼─────────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
  │ Memory instructions (loads pop addr, push value; stores pop addr+value)                │
  │ 0x28–0x35 loads, 0x36–0x3E stores, 0x3F–0x40 management                               │
  ├─────────┼─────────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
  │ Numeric, conversion: 0x41–0xBF                                                         │
  └─────────┴─────────────────────────┴──────────────┴────────────────────────┴─────┴──────┘
  ```

  The WASM 1.0 MVP defines exactly 172 instructions. Gaps in the byte range
  (e.g. 0x06–0x0A, 0x12–0x1F, 0x25–0x27) are reserved/unassigned.
  """

  # ────────────────────────────────────────────────────────────────────────────
  # Opcode table — module attribute evaluated at compile time
  #
  # Each entry is a map with keys:
  #   :name       — canonical text name (string)
  #   :opcode     — byte value (integer)
  #   :category   — instruction group (string)
  #   :immediates — list of immediate argument names (list of strings)
  #   :stack_pop  — values consumed from operand stack (integer)
  #   :stack_push — values produced onto operand stack (integer)
  #
  # The module attribute @opcodes is a plain Elixir list, evaluated once
  # when the module is compiled.  Elixir module attributes are compile-time
  # constants — they behave like C's `#define` but with full Elixir values.
  # ────────────────────────────────────────────────────────────────────────────

  @opcodes [
    # ── Control instructions ───────────────────────────────────────────────────
    #
    # Control instructions manage the program counter and structured control
    # flow. WASM has *no* unstructured jumps (unlike x86 `jmp`). All branches
    # target enclosing blocks identified by a label depth index.
    #
    # unreachable — unconditionally traps (like a failed assert / abort).
    # nop         — no operation; useful as a filler.
    # block       — opens a forward-jump target. `br` jumps past its `end`.
    # loop        — opens a backward-jump target. `br` jumps to its start.
    # if/else     — conditional; pops one i32 (0 = false, nonzero = true).
    # br          — unconditional branch to enclosing block at depth N.
    # br_if       — conditional branch; pops the condition i32.
    # br_table    — dispatch table: pops index, branches to matching label.
    # return      — branch to depth = function depth (exits the function).
    # call        — call a statically-known function by index.
    # call_indirect — call via the function table; pops i32 table index,
    #                 validates against typeidx, then calls. Enables C-style
    #                 function pointers and C++ vtable dispatch.
    %{name: "unreachable",   opcode: 0x00, category: "control",     immediates: [],                      stack_pop: 0, stack_push: 0},
    %{name: "nop",           opcode: 0x01, category: "control",     immediates: [],                      stack_pop: 0, stack_push: 0},
    %{name: "block",         opcode: 0x02, category: "control",     immediates: ["blocktype"],            stack_pop: 0, stack_push: 0},
    %{name: "loop",          opcode: 0x03, category: "control",     immediates: ["blocktype"],            stack_pop: 0, stack_push: 0},
    %{name: "if",            opcode: 0x04, category: "control",     immediates: ["blocktype"],            stack_pop: 1, stack_push: 0},
    %{name: "else",          opcode: 0x05, category: "control",     immediates: [],                      stack_pop: 0, stack_push: 0},
    %{name: "end",           opcode: 0x0B, category: "control",     immediates: [],                      stack_pop: 0, stack_push: 0},
    %{name: "br",            opcode: 0x0C, category: "control",     immediates: ["labelidx"],             stack_pop: 0, stack_push: 0},
    %{name: "br_if",         opcode: 0x0D, category: "control",     immediates: ["labelidx"],             stack_pop: 1, stack_push: 0},
    %{name: "br_table",      opcode: 0x0E, category: "control",     immediates: ["vec_labelidx"],         stack_pop: 1, stack_push: 0},
    %{name: "return",        opcode: 0x0F, category: "control",     immediates: [],                      stack_pop: 0, stack_push: 0},
    %{name: "call",          opcode: 0x10, category: "control",     immediates: ["funcidx"],              stack_pop: 0, stack_push: 0},
    %{name: "call_indirect", opcode: 0x11, category: "control",     immediates: ["typeidx", "tableidx"], stack_pop: 1, stack_push: 0},

    # ── Parametric instructions ───────────────────────────────────────────────
    #
    # drop   — discard the top stack value (any type).
    # select — like a C ternary: pops cond (i32), val2, val1;
    #          pushes val1 if cond != 0, else val2.
    #
    #   stack before select:  [..., val1, val2, cond]
    #   stack after  select:  [..., (cond != 0 ? val1 : val2)]
    %{name: "drop",   opcode: 0x1A, category: "parametric", immediates: [], stack_pop: 1, stack_push: 0},
    %{name: "select", opcode: 0x1B, category: "parametric", immediates: [], stack_pop: 3, stack_push: 1},

    # ── Variable instructions ──────────────────────────────────────────────────
    #
    # WASM functions have *local* variables (including parameters) indexed
    # from 0. The *global* index space covers imported globals then module globals.
    #
    # local.get  — push local[localidx] onto the stack.
    # local.set  — pop value, store into local[localidx].
    # local.tee  — store into local[localidx] WITHOUT popping (peek + set).
    # global.get — push global[globalidx].
    # global.set — pop value, store into mutable global[globalidx].
    %{name: "local.get",  opcode: 0x20, category: "variable", immediates: ["localidx"],  stack_pop: 0, stack_push: 1},
    %{name: "local.set",  opcode: 0x21, category: "variable", immediates: ["localidx"],  stack_pop: 1, stack_push: 0},
    %{name: "local.tee",  opcode: 0x22, category: "variable", immediates: ["localidx"],  stack_pop: 1, stack_push: 1},
    %{name: "global.get", opcode: 0x23, category: "variable", immediates: ["globalidx"], stack_pop: 0, stack_push: 1},
    %{name: "global.set", opcode: 0x24, category: "variable", immediates: ["globalidx"], stack_pop: 1, stack_push: 0},

    # ── Memory load instructions ───────────────────────────────────────────────
    #
    # All load instructions carry a `memarg` immediate: two LEB128 u32 values:
    #   align  — log₂ of expected alignment (a hint, not enforced)
    #   offset — static byte offset added to the dynamic address on the stack
    #
    # Effective address = stack.pop(i32) + offset.
    #
    # Suffix _s: sign-extend the narrow value to fill 32/64 bits.
    # Suffix _u: zero-extend (fill with zeros).
    #
    #   i32.load8_s 0x2C:  loads 1 byte, sign-extends → i32
    #   i64.load32_s 0x34: loads 4 bytes, sign-extends → i64
    %{name: "i32.load",    opcode: 0x28, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "i64.load",    opcode: 0x29, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "f32.load",    opcode: 0x2A, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "f64.load",    opcode: 0x2B, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "i32.load8_s", opcode: 0x2C, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "i32.load8_u", opcode: 0x2D, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "i32.load16_s",opcode: 0x2E, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "i32.load16_u",opcode: 0x2F, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "i64.load8_s", opcode: 0x30, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "i64.load8_u", opcode: 0x31, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "i64.load16_s",opcode: 0x32, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "i64.load16_u",opcode: 0x33, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "i64.load32_s",opcode: 0x34, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},
    %{name: "i64.load32_u",opcode: 0x35, category: "memory", immediates: ["memarg"], stack_pop: 1, stack_push: 1},

    # ── Memory store instructions ──────────────────────────────────────────────
    #
    # Store instructions pop TWO values: the address (i32) and the value.
    #   stack before i32.store: [..., addr: i32, value: i32]
    #   stack after:            [...]
    #
    # Narrow stores (store8, store16, store32) truncate the value to the
    # indicated bit width. No _s/_u distinction needed — truncation gives the
    # same low bits regardless of interpretation.
    %{name: "i32.store",   opcode: 0x36, category: "memory", immediates: ["memarg"], stack_pop: 2, stack_push: 0},
    %{name: "i64.store",   opcode: 0x37, category: "memory", immediates: ["memarg"], stack_pop: 2, stack_push: 0},
    %{name: "f32.store",   opcode: 0x38, category: "memory", immediates: ["memarg"], stack_pop: 2, stack_push: 0},
    %{name: "f64.store",   opcode: 0x39, category: "memory", immediates: ["memarg"], stack_pop: 2, stack_push: 0},
    %{name: "i32.store8",  opcode: 0x3A, category: "memory", immediates: ["memarg"], stack_pop: 2, stack_push: 0},
    %{name: "i32.store16", opcode: 0x3B, category: "memory", immediates: ["memarg"], stack_pop: 2, stack_push: 0},
    %{name: "i64.store8",  opcode: 0x3C, category: "memory", immediates: ["memarg"], stack_pop: 2, stack_push: 0},
    %{name: "i64.store16", opcode: 0x3D, category: "memory", immediates: ["memarg"], stack_pop: 2, stack_push: 0},
    %{name: "i64.store32", opcode: 0x3E, category: "memory", immediates: ["memarg"], stack_pop: 2, stack_push: 0},

    # ── Memory management ──────────────────────────────────────────────────────
    #
    # memory.size — push current memory size in pages (1 page = 64 KiB).
    # memory.grow — grow memory by N pages; push old size on success, -1 on fail.
    #
    # The memidx immediate is always 0 in WASM 1.0 (only one memory allowed).
    %{name: "memory.size", opcode: 0x3F, category: "memory", immediates: ["memidx"], stack_pop: 0, stack_push: 1},
    %{name: "memory.grow", opcode: 0x40, category: "memory", immediates: ["memidx"], stack_pop: 1, stack_push: 1},

    # ── i32 numeric instructions ───────────────────────────────────────────────
    #
    # WASM integers are *untyped bit patterns* — there is no distinct signed/
    # unsigned type. Signedness is a property of the *operation*, not the value.
    #
    #   i32.div_s — treats the i32 bits as two's-complement signed
    #   i32.div_u — treats the i32 bits as unsigned
    #   i32.lt_s  — signed less-than
    #   i32.lt_u  — unsigned less-than
    #
    # Boolean results are i32: 1 for true, 0 for false.
    # Bit operations (and/or/xor/shl/shr/rotl/rotr) are sign-agnostic.
    #
    # i32.eqz is unary (test-for-zero); all comparison operators are binary.
    #
    # Bit-counting:
    #   clz    — count leading  zeros (from most-significant bit)
    #   ctz    — count trailing zeros (from least-significant bit)
    #   popcnt — count set bits (Hamming weight / population count)
    %{name: "i32.const",  opcode: 0x41, category: "numeric_i32", immediates: ["i32"], stack_pop: 0, stack_push: 1},
    %{name: "i32.eqz",   opcode: 0x45, category: "numeric_i32", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "i32.eq",    opcode: 0x46, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.ne",    opcode: 0x47, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.lt_s",  opcode: 0x48, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.lt_u",  opcode: 0x49, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.gt_s",  opcode: 0x4A, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.gt_u",  opcode: 0x4B, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.le_s",  opcode: 0x4C, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.le_u",  opcode: 0x4D, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.ge_s",  opcode: 0x4E, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.ge_u",  opcode: 0x4F, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.clz",   opcode: 0x67, category: "numeric_i32", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "i32.ctz",   opcode: 0x68, category: "numeric_i32", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "i32.popcnt",opcode: 0x69, category: "numeric_i32", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "i32.add",   opcode: 0x6A, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.sub",   opcode: 0x6B, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.mul",   opcode: 0x6C, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.div_s", opcode: 0x6D, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.div_u", opcode: 0x6E, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.rem_s", opcode: 0x6F, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.rem_u", opcode: 0x70, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.and",   opcode: 0x71, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.or",    opcode: 0x72, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.xor",   opcode: 0x73, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.shl",   opcode: 0x74, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.shr_s", opcode: 0x75, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.shr_u", opcode: 0x76, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.rotl",  opcode: 0x77, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i32.rotr",  opcode: 0x78, category: "numeric_i32", immediates: [],      stack_pop: 2, stack_push: 1},

    # ── i64 numeric instructions ───────────────────────────────────────────────
    #
    # Mirror of the i32 set but operating on 64-bit integers.
    # All signed/unsigned and bit-counting notes from i32 apply here too.
    %{name: "i64.const",  opcode: 0x42, category: "numeric_i64", immediates: ["i64"], stack_pop: 0, stack_push: 1},
    %{name: "i64.eqz",   opcode: 0x50, category: "numeric_i64", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "i64.eq",    opcode: 0x51, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.ne",    opcode: 0x52, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.lt_s",  opcode: 0x53, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.lt_u",  opcode: 0x54, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.gt_s",  opcode: 0x55, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.gt_u",  opcode: 0x56, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.le_s",  opcode: 0x57, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.le_u",  opcode: 0x58, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.ge_s",  opcode: 0x59, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.ge_u",  opcode: 0x5A, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.clz",   opcode: 0x79, category: "numeric_i64", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "i64.ctz",   opcode: 0x7A, category: "numeric_i64", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "i64.popcnt",opcode: 0x7B, category: "numeric_i64", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "i64.add",   opcode: 0x7C, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.sub",   opcode: 0x7D, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.mul",   opcode: 0x7E, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.div_s", opcode: 0x7F, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.div_u", opcode: 0x80, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.rem_s", opcode: 0x81, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.rem_u", opcode: 0x82, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.and",   opcode: 0x83, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.or",    opcode: 0x84, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.xor",   opcode: 0x85, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.shl",   opcode: 0x86, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.shr_s", opcode: 0x87, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.shr_u", opcode: 0x88, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.rotl",  opcode: 0x89, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "i64.rotr",  opcode: 0x8A, category: "numeric_i64", immediates: [],      stack_pop: 2, stack_push: 1},

    # ── f32 numeric instructions ───────────────────────────────────────────────
    #
    # IEEE 754 single-precision (32-bit) floating-point instructions.
    #
    # Comparison results are i32 (1 = true, 0 = false).  NaN comparisons
    # always return 0 (false) — this matches IEEE 754 semantics where NaN is
    # "unordered" with respect to every value including itself.
    #
    # f32.nearest rounds to the nearest integer with ties-to-even
    # (banker's rounding), matching IEEE 754 "roundTiesToEven" mode.
    # f32.copysign copies the sign bit from the second operand to the first.
    %{name: "f32.const",   opcode: 0x43, category: "numeric_f32", immediates: ["f32"], stack_pop: 0, stack_push: 1},
    %{name: "f32.eq",      opcode: 0x5B, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.ne",      opcode: 0x5C, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.lt",      opcode: 0x5D, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.gt",      opcode: 0x5E, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.le",      opcode: 0x5F, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.ge",      opcode: 0x60, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.abs",     opcode: 0x8B, category: "numeric_f32", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f32.neg",     opcode: 0x8C, category: "numeric_f32", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f32.ceil",    opcode: 0x8D, category: "numeric_f32", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f32.floor",   opcode: 0x8E, category: "numeric_f32", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f32.trunc",   opcode: 0x8F, category: "numeric_f32", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f32.nearest", opcode: 0x90, category: "numeric_f32", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f32.sqrt",    opcode: 0x91, category: "numeric_f32", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f32.add",     opcode: 0x92, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.sub",     opcode: 0x93, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.mul",     opcode: 0x94, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.div",     opcode: 0x95, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.min",     opcode: 0x96, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.max",     opcode: 0x97, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f32.copysign",opcode: 0x98, category: "numeric_f32", immediates: [],      stack_pop: 2, stack_push: 1},

    # ── f64 numeric instructions ───────────────────────────────────────────────
    #
    # IEEE 754 double-precision (64-bit) floating-point. Mirror of f32 set.
    %{name: "f64.const",   opcode: 0x44, category: "numeric_f64", immediates: ["f64"], stack_pop: 0, stack_push: 1},
    %{name: "f64.eq",      opcode: 0x61, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.ne",      opcode: 0x62, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.lt",      opcode: 0x63, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.gt",      opcode: 0x64, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.le",      opcode: 0x65, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.ge",      opcode: 0x66, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.abs",     opcode: 0x99, category: "numeric_f64", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f64.neg",     opcode: 0x9A, category: "numeric_f64", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f64.ceil",    opcode: 0x9B, category: "numeric_f64", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f64.floor",   opcode: 0x9C, category: "numeric_f64", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f64.trunc",   opcode: 0x9D, category: "numeric_f64", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f64.nearest", opcode: 0x9E, category: "numeric_f64", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f64.sqrt",    opcode: 0x9F, category: "numeric_f64", immediates: [],      stack_pop: 1, stack_push: 1},
    %{name: "f64.add",     opcode: 0xA0, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.sub",     opcode: 0xA1, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.mul",     opcode: 0xA2, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.div",     opcode: 0xA3, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.min",     opcode: 0xA4, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.max",     opcode: 0xA5, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},
    %{name: "f64.copysign",opcode: 0xA6, category: "numeric_f64", immediates: [],      stack_pop: 2, stack_push: 1},

    # ── Conversion instructions ────────────────────────────────────────────────
    #
    # Conversions change the type of a value.  All are unary (pop 1, push 1).
    # Naming pattern: <dest_type>.<operation>_<source_type>
    #
    #   wrap       — truncate i64 → i32 (just drops the high 32 bits)
    #   extend     — widen i32 → i64, with explicit sign: _s (sign-extend) or _u (zero-extend)
    #   trunc      — float → int by truncation toward zero; traps on NaN/infinity
    #   convert    — int → float (may lose precision for large i64 values)
    #   demote     — f64 → f32 (may lose precision)
    #   promote    — f32 → f64 (exact, no precision loss)
    #   reinterpret— same 32/64 bit pattern, different WASM type
    #
    # Reinterpret examples:
    #   i32.reinterpret_f32: treats the 4 bytes of an f32 as an i32 bit pattern
    #   f32.reinterpret_i32: the reverse — no arithmetic change, just re-labels
    %{name: "i32.wrap_i64",       opcode: 0xA7, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i32.trunc_f32_s",    opcode: 0xA8, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i32.trunc_f32_u",    opcode: 0xA9, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i32.trunc_f64_s",    opcode: 0xAA, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i32.trunc_f64_u",    opcode: 0xAB, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i64.extend_i32_s",   opcode: 0xAC, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i64.extend_i32_u",   opcode: 0xAD, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i64.trunc_f32_s",    opcode: 0xAE, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i64.trunc_f32_u",    opcode: 0xAF, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i64.trunc_f64_s",    opcode: 0xB0, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i64.trunc_f64_u",    opcode: 0xB1, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f32.convert_i32_s",  opcode: 0xB2, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f32.convert_i32_u",  opcode: 0xB3, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f32.convert_i64_s",  opcode: 0xB4, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f32.convert_i64_u",  opcode: 0xB5, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f32.demote_f64",     opcode: 0xB6, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f64.convert_i32_s",  opcode: 0xB7, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f64.convert_i32_u",  opcode: 0xB8, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f64.convert_i64_s",  opcode: 0xB9, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f64.convert_i64_u",  opcode: 0xBA, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f64.promote_f32",    opcode: 0xBB, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i32.reinterpret_f32",opcode: 0xBC, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "i64.reinterpret_f64",opcode: 0xBD, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f32.reinterpret_i32",opcode: 0xBE, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
    %{name: "f64.reinterpret_i64",opcode: 0xBF, category: "conversion", immediates: [], stack_pop: 1, stack_push: 1},
  ]

  # ────────────────────────────────────────────────────────────────────────────
  # Compile-time maps
  #
  # We build two lookup maps from @opcodes at compile time using module
  # attributes. In Elixir, a module attribute evaluated to a complex value
  # (like a Map) is computed once when the module is compiled and stored as a
  # constant in the BEAM bytecode — zero runtime cost for map creation.
  #
  # @opcodes_by_byte   :: %{integer => map}  — keyed by opcode byte
  # @opcodes_by_name   :: %{string  => map}  — keyed by instruction name
  # ────────────────────────────────────────────────────────────────────────────

  @opcodes_by_byte Map.new(@opcodes, fn entry -> {entry.opcode, entry} end)

  @opcodes_by_name Map.new(@opcodes, fn entry -> {entry.name, entry} end)

  # ────────────────────────────────────────────────────────────────────────────
  # Public API
  # ────────────────────────────────────────────────────────────────────────────

  @doc """
  Look up an opcode by its byte value.

  Returns `{:ok, opcode_map}` for any defined WASM 1.0 opcode, or
  `{:error, :unknown_opcode}` for undefined/reserved bytes.

  ## Examples

      iex> {:ok, op} = CodingAdventures.WasmOpcodes.get_opcode(0x6A)
      iex> op.name
      "i32.add"

      iex> CodingAdventures.WasmOpcodes.get_opcode(0xFF)
      {:error, :unknown_opcode}

  """
  @spec get_opcode(byte()) :: {:ok, map()} | {:error, :unknown_opcode}
  def get_opcode(byte) do
    # Map.fetch/2 returns {:ok, value} or :error; we normalise :error to our
    # tagged form so callers always get a two-element tuple back.
    case Map.fetch(@opcodes_by_byte, byte) do
      {:ok, opcode_map} -> {:ok, opcode_map}
      :error -> {:error, :unknown_opcode}
    end
  end

  @doc """
  Look up an opcode by its canonical text name.

  Names are case-sensitive and use WASM text format notation,
  e.g. `"i32.add"`, `"call_indirect"`, `"f64.reinterpret_i64"`.

  Returns `{:ok, opcode_map}` on success, `{:error, :unknown_opcode}` if
  the name is not found.

  ## Examples

      iex> {:ok, op} = CodingAdventures.WasmOpcodes.get_opcode_by_name("i32.add")
      iex> op.opcode
      0x6A

      iex> CodingAdventures.WasmOpcodes.get_opcode_by_name("banana")
      {:error, :unknown_opcode}

  """
  @spec get_opcode_by_name(String.t()) :: {:ok, map()} | {:error, :unknown_opcode}
  def get_opcode_by_name(name) do
    case Map.fetch(@opcodes_by_name, name) do
      {:ok, opcode_map} -> {:ok, opcode_map}
      :error -> {:error, :unknown_opcode}
    end
  end

  @doc """
  Return all opcodes as a list of maps.

  Each map has keys: `:name`, `:opcode`, `:category`, `:immediates`,
  `:stack_pop`, `:stack_push`.

  The order is the same as the source definition (control first, then
  parametric, variable, memory, numeric, conversion).

  ## Example

      iex> ops = CodingAdventures.WasmOpcodes.all_opcodes()
      iex> length(ops) >= 172
      true

  """
  @spec all_opcodes() :: [map()]
  def all_opcodes do
    @opcodes
  end
end
