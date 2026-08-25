//! # wasm-opcodes
//!
//! Complete WASM 1.0 opcode lookup table with metadata for every instruction.
//!
//! This crate is part of the coding-adventures monorepo — a ground-up
//! implementation of the computing stack from transistors to operating systems.
//!
//! ## What is a WASM opcode?
//!
//! A WebAssembly binary is a sequence of *sections*. The code section holds
//! function bodies, each of which is a flat byte sequence of *instructions*.
//! The first byte of every instruction is its **opcode** — a single-byte
//! value in WASM 1.0's core range (0x00–0xBF) plus the widely-implemented,
//! still-single-byte "sign-extension operators" proposal (0xC0–0xC4). True
//! multi-byte opcodes via the 0xFC prefix (the "non-trapping float-to-int
//! conversions" proposal, SIMD, etc.) are out of scope for this table —
//! callers that need those (`wasm-wast-parser`'s instruction encoder,
//! `wasm-execution`'s decoder) special-case the 0xFC prefix byte directly.
//!
//! ```text
//! Function body byte stream example:
//!
//!   0x20 0x00      ← local.get  $local_0
//!   0x20 0x01      ← local.get  $local_1
//!   0x6A           ← i32.add
//!   0x0F           ← return
//!   0x0B           ← end
//! ```
//!
//! ## The operand stack
//!
//! WASM is a **stack machine**. Instructions consume values from a virtual
//! operand stack (stack_pop) and push results back onto it (stack_push).
//!
//! ```text
//! Before i32.add:   [..., 3, 7]
//! After  i32.add:   [..., 10]     ← popped 2, pushed 1
//! ```
//!
//! The `stack_pop` and `stack_push` fields encode this for each instruction.
//! For control instructions (block/loop/if/call) these are 0/0 because the
//! actual arity depends on the block type or function signature — the fields
//! track *structural* pops/pushes from the static table, not runtime effects.
//!
//! ## Immediates
//!
//! Many instructions carry **immediate** arguments encoded directly in the
//! byte stream right after the opcode byte. For example:
//!
//! ```text
//! Instruction          Immediates
//! ─────────────────────────────────────────────
//! local.get $0         localidx  (LEB128 u32)
//! i32.const 42         i32       (signed LEB128)
//! i32.load offset=8    memarg    (align:u32, offset:u32)
//! br_table [0,1,2] 3   vec_labelidx (count + labels + default)
//! ```
//!
//! The `immediates` field is a slice of string names describing what follows
//! the opcode byte in the binary.
//!
//! ## Complete WASM 1.0 opcode table (183 entries)
//!
//! ```text
//! ┌─────────┬──────────────────────┬──────────────┬────────────────────────┬─────┬──────┐
//! │ Opcode  │ Name                 │ Category     │ Immediates             │ Pop │ Push │
//! ├─────────┼──────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
//! │ Control instructions                                                                 │
//! │ 0x00    │ unreachable          │ control      │ —                      │  0  │  0   │
//! │ 0x01    │ nop                  │ control      │ —                      │  0  │  0   │
//! │ 0x02    │ block                │ control      │ blocktype              │  0  │  0   │
//! │ 0x03    │ loop                 │ control      │ blocktype              │  0  │  0   │
//! │ 0x04    │ if                   │ control      │ blocktype              │  1  │  0   │
//! │ 0x05    │ else                 │ control      │ —                      │  0  │  0   │
//! │ 0x0B    │ end                  │ control      │ —                      │  0  │  0   │
//! │ 0x0C    │ br                   │ control      │ labelidx               │  0  │  0   │
//! │ 0x0D    │ br_if                │ control      │ labelidx               │  1  │  0   │
//! │ 0x0E    │ br_table             │ control      │ vec_labelidx           │  1  │  0   │
//! │ 0x0F    │ return               │ control      │ —                      │  0  │  0   │
//! │ 0x10    │ call                 │ control      │ funcidx                │  0  │  0   │
//! │ 0x11    │ call_indirect        │ control      │ typeidx, tableidx      │  1  │  0   │
//! │ 0x12    │ return_call          │ control      │ funcidx                │  0  │  0   │
//! │ 0x13    │ return_call_indirect │ control      │ typeidx, tableidx      │  1  │  0   │
//! ├─────────┼──────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
//! │ Parametric instructions                                                              │
//! │ 0x1A    │ drop                 │ parametric   │ —                      │  1  │  0   │
//! │ 0x1B    │ select               │ parametric   │ —                      │  3  │  1   │
//! ├─────────┼──────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
//! │ Variable instructions                                                                │
//! │ 0x20    │ local.get            │ variable     │ localidx               │  0  │  1   │
//! │ 0x21    │ local.set            │ variable     │ localidx               │  1  │  0   │
//! │ 0x22    │ local.tee            │ variable     │ localidx               │  1  │  1   │
//! │ 0x23    │ global.get           │ variable     │ globalidx              │  0  │  1   │
//! │ 0x24    │ global.set           │ variable     │ globalidx              │  1  │  0   │
//! ├─────────┼──────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
//! │ Memory load instructions (memarg = align:u32, offset:u32)                           │
//! │ 0x28    │ i32.load             │ memory       │ memarg                 │  1  │  1   │
//! │ 0x29    │ i64.load             │ memory       │ memarg                 │  1  │  1   │
//! │ 0x2A    │ f32.load             │ memory       │ memarg                 │  1  │  1   │
//! │ 0x2B    │ f64.load             │ memory       │ memarg                 │  1  │  1   │
//! │ 0x2C    │ i32.load8_s          │ memory       │ memarg                 │  1  │  1   │
//! │ 0x2D    │ i32.load8_u          │ memory       │ memarg                 │  1  │  1   │
//! │ 0x2E    │ i32.load16_s         │ memory       │ memarg                 │  1  │  1   │
//! │ 0x2F    │ i32.load16_u         │ memory       │ memarg                 │  1  │  1   │
//! │ 0x30    │ i64.load8_s          │ memory       │ memarg                 │  1  │  1   │
//! │ 0x31    │ i64.load8_u          │ memory       │ memarg                 │  1  │  1   │
//! │ 0x32    │ i64.load16_s         │ memory       │ memarg                 │  1  │  1   │
//! │ 0x33    │ i64.load16_u         │ memory       │ memarg                 │  1  │  1   │
//! │ 0x34    │ i64.load32_s         │ memory       │ memarg                 │  1  │  1   │
//! │ 0x35    │ i64.load32_u         │ memory       │ memarg                 │  1  │  1   │
//! ├─────────┼──────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
//! │ Memory store instructions (memarg = align:u32, offset:u32)                          │
//! │ 0x36    │ i32.store            │ memory       │ memarg                 │  2  │  0   │
//! │ 0x37    │ i64.store            │ memory       │ memarg                 │  2  │  0   │
//! │ 0x38    │ f32.store            │ memory       │ memarg                 │  2  │  0   │
//! │ 0x39    │ f64.store            │ memory       │ memarg                 │  2  │  0   │
//! │ 0x3A    │ i32.store8           │ memory       │ memarg                 │  2  │  0   │
//! │ 0x3B    │ i32.store16          │ memory       │ memarg                 │  2  │  0   │
//! │ 0x3C    │ i64.store8           │ memory       │ memarg                 │  2  │  0   │
//! │ 0x3D    │ i64.store16          │ memory       │ memarg                 │  2  │  0   │
//! │ 0x3E    │ i64.store32          │ memory       │ memarg                 │  2  │  0   │
//! ├─────────┼──────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
//! │ Memory management                                                                    │
//! │ 0x3F    │ memory.size          │ memory       │ memidx                 │  0  │  1   │
//! │ 0x40    │ memory.grow          │ memory       │ memidx                 │  1  │  1   │
//! ├─────────┼──────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
//! │ i32 numeric instructions                                                             │
//! │ 0x41    │ i32.const            │ numeric_i32  │ i32                    │  0  │  1   │
//! │ 0x45    │ i32.eqz              │ numeric_i32  │ —                      │  1  │  1   │
//! │ 0x46    │ i32.eq               │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x47    │ i32.ne               │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x48    │ i32.lt_s             │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x49    │ i32.lt_u             │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x4A    │ i32.gt_s             │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x4B    │ i32.gt_u             │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x4C    │ i32.le_s             │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x4D    │ i32.le_u             │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x4E    │ i32.ge_s             │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x4F    │ i32.ge_u             │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x67    │ i32.clz              │ numeric_i32  │ —                      │  1  │  1   │
//! │ 0x68    │ i32.ctz              │ numeric_i32  │ —                      │  1  │  1   │
//! │ 0x69    │ i32.popcnt           │ numeric_i32  │ —                      │  1  │  1   │
//! │ 0x6A    │ i32.add              │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x6B    │ i32.sub              │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x6C    │ i32.mul              │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x6D    │ i32.div_s            │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x6E    │ i32.div_u            │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x6F    │ i32.rem_s            │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x70    │ i32.rem_u            │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x71    │ i32.and              │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x72    │ i32.or               │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x73    │ i32.xor              │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x74    │ i32.shl              │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x75    │ i32.shr_s            │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x76    │ i32.shr_u            │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x77    │ i32.rotl             │ numeric_i32  │ —                      │  2  │  1   │
//! │ 0x78    │ i32.rotr             │ numeric_i32  │ —                      │  2  │  1   │
//! ├─────────┼──────────────────────┼──────────────┼────────────────────────┼─────┼──────┤
//! │ i64 numeric instructions                                                             │
//! │ 0x42    │ i64.const            │ numeric_i64  │ i64                    │  0  │  1   │
//! │ 0x50    │ i64.eqz              │ numeric_i64  │ —                      │  1  │  1   │
//! │ 0x51    │ i64.eq               │ numeric_i64  │ —                      │  2  │  1   │
//! │ ... (full table continues) ...                                                       │
//! └─────────┴──────────────────────┴──────────────┴────────────────────────┴─────┴──────┘
//! ```

// ──────────────────────────────────────────────────────────────────────────────
// OpcodeInfo — the core data structure
// ──────────────────────────────────────────────────────────────────────────────

/// Metadata for a single WASM 1.0 instruction.
///
/// All fields use `&'static str` / `&'static [&'static str]` so the entire
/// table can live in read-only memory (`.rodata`) without any heap allocation.
///
/// # Fields
/// - `name`       — canonical text name (e.g., `"i32.add"`)
/// - `opcode`     — the byte value (e.g., `0x6A`)
/// - `category`   — instruction group (e.g., `"numeric_i32"`)
/// - `immediates` — names of immediate arguments that follow the opcode byte
/// - `stack_pop`  — number of values consumed from the operand stack
/// - `stack_push` — number of values produced onto the operand stack
///
/// Note: for control instructions (call, block, if, etc.) stack_pop/push are
/// the *structural* counts from the static opcode definition.  The true runtime
/// arity depends on the function type or block type referenced by the immediate.
#[derive(Debug, Clone, PartialEq)]
pub struct OpcodeInfo {
    pub name: &'static str,
    pub opcode: u8,
    pub category: &'static str,
    pub immediates: &'static [&'static str],
    pub stack_pop: u8,
    pub stack_push: u8,
}

// ──────────────────────────────────────────────────────────────────────────────
// Static opcode table — all 183 WASM 1.0 instructions
//
// Ordered by opcode byte for readability. The lookup functions do a linear
// scan, which is perfectly fine for 183 entries (~183 comparisons worst case,
// negligible on modern hardware).
// ──────────────────────────────────────────────────────────────────────────────

/// Sorted slice of all WASM 1.0 opcodes. Used as the authoritative source for
/// both `get_opcode` and `get_opcode_by_name`.
pub static OPCODES: &[OpcodeInfo] = &[
    // ── Control instructions ──────────────────────────────────────────────────
    //
    // Control instructions manage the program counter and structured control
    // flow.  WASM has *no* unstructured jumps (unlike x86 `jmp`).  All branches
    // target enclosing blocks identified by a label depth index.
    //
    // `unreachable` — unconditionally traps the program (like a failed assert).
    // `nop`         — does nothing; useful as a placeholder.
    // `block`       — opens a forward-jump target; `br` jumps past its `end`.
    // `loop`        — opens a backward-jump target; `br` jumps to its start.
    // `if`/`else`   — conditional; pops one i32 (0 = false, nonzero = true).
    // `br`          — unconditional branch to label at depth N.
    // `br_if`       — conditional branch; pops the condition i32.
    // `br_table`    — dispatch table; pops index, branches to matching label.
    // `return`      — branch to depth = function depth (exits function).
    // `call`        — call a statically-known function by index.
    // `call_indirect` — call a dynamically-selected function via the table;
    //                   pops the i32 table index, then validates against typeidx.
    OpcodeInfo { name: "unreachable",   opcode: 0x00, category: "control",     immediates: &[],                            stack_pop: 0, stack_push: 0 },
    OpcodeInfo { name: "nop",           opcode: 0x01, category: "control",     immediates: &[],                            stack_pop: 0, stack_push: 0 },
    OpcodeInfo { name: "block",         opcode: 0x02, category: "control",     immediates: &["blocktype"],                 stack_pop: 0, stack_push: 0 },
    OpcodeInfo { name: "loop",          opcode: 0x03, category: "control",     immediates: &["blocktype"],                 stack_pop: 0, stack_push: 0 },
    OpcodeInfo { name: "if",            opcode: 0x04, category: "control",     immediates: &["blocktype"],                 stack_pop: 1, stack_push: 0 },
    OpcodeInfo { name: "else",          opcode: 0x05, category: "control",     immediates: &[],                            stack_pop: 0, stack_push: 0 },
    OpcodeInfo { name: "end",           opcode: 0x0B, category: "control",     immediates: &[],                            stack_pop: 0, stack_push: 0 },
    OpcodeInfo { name: "br",            opcode: 0x0C, category: "control",     immediates: &["labelidx"],                  stack_pop: 0, stack_push: 0 },
    OpcodeInfo { name: "br_if",         opcode: 0x0D, category: "control",     immediates: &["labelidx"],                  stack_pop: 1, stack_push: 0 },
    OpcodeInfo { name: "br_table",      opcode: 0x0E, category: "control",     immediates: &["vec_labelidx"],              stack_pop: 1, stack_push: 0 },
    OpcodeInfo { name: "return",        opcode: 0x0F, category: "control",     immediates: &[],                            stack_pop: 0, stack_push: 0 },
    OpcodeInfo { name: "call",          opcode: 0x10, category: "control",     immediates: &["funcidx"],                   stack_pop: 0, stack_push: 0 },
    OpcodeInfo { name: "call_indirect", opcode: 0x11, category: "control",     immediates: &["typeidx", "tableidx"],       stack_pop: 1, stack_push: 0 },
    OpcodeInfo { name: "return_call",          opcode: 0x12, category: "control", immediates: &["funcidx"],             stack_pop: 0, stack_push: 0 },
    OpcodeInfo { name: "return_call_indirect", opcode: 0x13, category: "control", immediates: &["typeidx", "tableidx"], stack_pop: 1, stack_push: 0 },

    // ── Parametric instructions ───────────────────────────────────────────────
    //
    // `drop`   — discard the top stack value (any type).
    // `select` — like a C ternary: pops condition (i32), val2, val1;
    //            pushes val1 if condition != 0, else val2.
    //
    //   stack before select:  [..., val1, val2, cond]
    //   stack after  select:  [..., (cond ? val1 : val2)]
    OpcodeInfo { name: "drop",   opcode: 0x1A, category: "parametric", immediates: &[], stack_pop: 1, stack_push: 0 },
    OpcodeInfo { name: "select", opcode: 0x1B, category: "parametric", immediates: &[], stack_pop: 3, stack_push: 1 },

    // ── Variable instructions ─────────────────────────────────────────────────
    //
    // WASM functions have *local* variables (including parameters) indexed 0..N-1.
    // The *global* index space covers imported globals followed by module globals.
    //
    // `local.get`  — push local[localidx] onto the stack.
    // `local.set`  — pop value, store into local[localidx].
    // `local.tee`  — store into local[localidx] WITHOUT popping (peek + set).
    // `global.get` — push global[globalidx].
    // `global.set` — pop value, store into mutable global[globalidx].
    OpcodeInfo { name: "local.get",  opcode: 0x20, category: "variable", immediates: &["localidx"],  stack_pop: 0, stack_push: 1 },
    OpcodeInfo { name: "local.set",  opcode: 0x21, category: "variable", immediates: &["localidx"],  stack_pop: 1, stack_push: 0 },
    OpcodeInfo { name: "local.tee",  opcode: 0x22, category: "variable", immediates: &["localidx"],  stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "global.get", opcode: 0x23, category: "variable", immediates: &["globalidx"], stack_pop: 0, stack_push: 1 },
    OpcodeInfo { name: "global.set", opcode: 0x24, category: "variable", immediates: &["globalidx"], stack_pop: 1, stack_push: 0 },

    // ── Table instructions (reference-types proposal, WASM17) ────────────────
    //
    // `table.get` — push table[tableidx][index] (a funcref, currently the
    //   only element type WASM 1.0's single table can hold).
    // `table.set` — pop a funcref and an i32 index, store into
    //   table[tableidx][index].
    // Both take a single `tableidx` LEB128 immediate, same shape as
    // `global.get`/`global.set`'s `globalidx` immediate above. See
    // `code/specs/W08-wasm-funcref-externref.md`.
    OpcodeInfo { name: "table.get", opcode: 0x25, category: "table", immediates: &["tableidx"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "table.set", opcode: 0x26, category: "table", immediates: &["tableidx"], stack_pop: 2, stack_push: 0 },

    // ── Memory load instructions ──────────────────────────────────────────────
    //
    // All load instructions have a `memarg` immediate: two LEB128 u32 values:
    //   - `align`  — log2 of the expected alignment (hint, not enforced)
    //   - `offset` — static byte offset added to the dynamic address
    //
    // The effective address = stack_top(i32) + offset.
    //
    // Loads that end in `_s` sign-extend the narrow value into 32/64 bits.
    // Loads that end in `_u` zero-extend.
    //
    //   i32.load8_s 0x2C:  loads 1 byte, sign-extends to i32
    //   i64.load32_s 0x34: loads 4 bytes, sign-extends to i64
    OpcodeInfo { name: "i32.load",    opcode: 0x28, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.load",    opcode: 0x29, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.load",    opcode: 0x2A, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.load",    opcode: 0x2B, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.load8_s", opcode: 0x2C, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.load8_u", opcode: 0x2D, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.load16_s",opcode: 0x2E, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.load16_u",opcode: 0x2F, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.load8_s", opcode: 0x30, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.load8_u", opcode: 0x31, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.load16_s",opcode: 0x32, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.load16_u",opcode: 0x33, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.load32_s",opcode: 0x34, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.load32_u",opcode: 0x35, category: "memory", immediates: &["memarg"], stack_pop: 1, stack_push: 1 },

    // ── Memory store instructions ─────────────────────────────────────────────
    //
    // Store instructions pop TWO values: the address (i32) and the value.
    // Stack before i32.store: [..., addr: i32, value: i32]
    // Stack after:            [...]
    //
    // Narrow stores (store8, store16, store32) truncate the value to the
    // indicated width before writing to memory.  No _s/_u suffix needed —
    // truncation has the same bit pattern regardless of signedness.
    OpcodeInfo { name: "i32.store",   opcode: 0x36, category: "memory", immediates: &["memarg"], stack_pop: 2, stack_push: 0 },
    OpcodeInfo { name: "i64.store",   opcode: 0x37, category: "memory", immediates: &["memarg"], stack_pop: 2, stack_push: 0 },
    OpcodeInfo { name: "f32.store",   opcode: 0x38, category: "memory", immediates: &["memarg"], stack_pop: 2, stack_push: 0 },
    OpcodeInfo { name: "f64.store",   opcode: 0x39, category: "memory", immediates: &["memarg"], stack_pop: 2, stack_push: 0 },
    OpcodeInfo { name: "i32.store8",  opcode: 0x3A, category: "memory", immediates: &["memarg"], stack_pop: 2, stack_push: 0 },
    OpcodeInfo { name: "i32.store16", opcode: 0x3B, category: "memory", immediates: &["memarg"], stack_pop: 2, stack_push: 0 },
    OpcodeInfo { name: "i64.store8",  opcode: 0x3C, category: "memory", immediates: &["memarg"], stack_pop: 2, stack_push: 0 },
    OpcodeInfo { name: "i64.store16", opcode: 0x3D, category: "memory", immediates: &["memarg"], stack_pop: 2, stack_push: 0 },
    OpcodeInfo { name: "i64.store32", opcode: 0x3E, category: "memory", immediates: &["memarg"], stack_pop: 2, stack_push: 0 },

    // ── Memory management ────────────────────────────────────────────────────-
    //
    // `memory.size` — push the current memory size in pages (1 page = 64 KiB).
    // `memory.grow` — attempt to grow memory by N pages; pushes old size on
    //                 success, -1 (as i32) on failure.
    //
    // The `memidx` immediate is always 0 in WASM 1.0 (only one memory allowed).
    OpcodeInfo { name: "memory.size", opcode: 0x3F, category: "memory", immediates: &["memidx"], stack_pop: 0, stack_push: 1 },
    OpcodeInfo { name: "memory.grow", opcode: 0x40, category: "memory", immediates: &["memidx"], stack_pop: 1, stack_push: 1 },

    // ── i32 numeric instructions ──────────────────────────────────────────────
    //
    // WASM integers are *untyped bit patterns* — there is no separate signed/
    // unsigned integer type.  Signedness is a property of the *operation*:
    //
    //   i32.div_s — treats the i32 bits as two's complement signed
    //   i32.div_u — treats the i32 bits as unsigned
    //   i32.lt_s  — signed less-than
    //   i32.lt_u  — unsigned less-than
    //
    // Boolean results are i32: 1 for true, 0 for false.
    //
    // Bit operations (and/or/xor/shl/shr/rotl/rotr) are sign-agnostic.
    //
    // `i32.eqz` is a unary operator (test-for-zero); all comparison operators
    // are binary (pop two, push one bool-as-i32).
    //
    // Bit-counting instructions:
    //   `clz`    — count leading zeros  (most-significant side)
    //   `ctz`    — count trailing zeros (least-significant side)
    //   `popcnt` — count set bits (Hamming weight)
    OpcodeInfo { name: "i32.const",  opcode: 0x41, category: "numeric_i32", immediates: &["i32"], stack_pop: 0, stack_push: 1 },
    OpcodeInfo { name: "i32.eqz",   opcode: 0x45, category: "numeric_i32", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.eq",    opcode: 0x46, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.ne",    opcode: 0x47, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.lt_s",  opcode: 0x48, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.lt_u",  opcode: 0x49, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.gt_s",  opcode: 0x4A, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.gt_u",  opcode: 0x4B, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.le_s",  opcode: 0x4C, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.le_u",  opcode: 0x4D, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.ge_s",  opcode: 0x4E, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.ge_u",  opcode: 0x4F, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.clz",   opcode: 0x67, category: "numeric_i32", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.ctz",   opcode: 0x68, category: "numeric_i32", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.popcnt",opcode: 0x69, category: "numeric_i32", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.add",   opcode: 0x6A, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.sub",   opcode: 0x6B, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.mul",   opcode: 0x6C, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.div_s", opcode: 0x6D, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.div_u", opcode: 0x6E, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.rem_s", opcode: 0x6F, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.rem_u", opcode: 0x70, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.and",   opcode: 0x71, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.or",    opcode: 0x72, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.xor",   opcode: 0x73, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.shl",   opcode: 0x74, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.shr_s", opcode: 0x75, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.shr_u", opcode: 0x76, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.rotl",  opcode: 0x77, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i32.rotr",  opcode: 0x78, category: "numeric_i32", immediates: &[],      stack_pop: 2, stack_push: 1 },

    // ── i64 numeric instructions ──────────────────────────────────────────────
    //
    // Mirror of the i32 set but operating on 64-bit integers.
    // All the same signedness notes apply.
    OpcodeInfo { name: "i64.const",  opcode: 0x42, category: "numeric_i64", immediates: &["i64"], stack_pop: 0, stack_push: 1 },
    OpcodeInfo { name: "i64.eqz",   opcode: 0x50, category: "numeric_i64", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.eq",    opcode: 0x51, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.ne",    opcode: 0x52, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.lt_s",  opcode: 0x53, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.lt_u",  opcode: 0x54, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.gt_s",  opcode: 0x55, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.gt_u",  opcode: 0x56, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.le_s",  opcode: 0x57, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.le_u",  opcode: 0x58, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.ge_s",  opcode: 0x59, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.ge_u",  opcode: 0x5A, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.clz",   opcode: 0x79, category: "numeric_i64", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.ctz",   opcode: 0x7A, category: "numeric_i64", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.popcnt",opcode: 0x7B, category: "numeric_i64", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.add",   opcode: 0x7C, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.sub",   opcode: 0x7D, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.mul",   opcode: 0x7E, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.div_s", opcode: 0x7F, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.div_u", opcode: 0x80, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.rem_s", opcode: 0x81, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.rem_u", opcode: 0x82, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.and",   opcode: 0x83, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.or",    opcode: 0x84, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.xor",   opcode: 0x85, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.shl",   opcode: 0x86, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.shr_s", opcode: 0x87, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.shr_u", opcode: 0x88, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.rotl",  opcode: 0x89, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "i64.rotr",  opcode: 0x8A, category: "numeric_i64", immediates: &[],      stack_pop: 2, stack_push: 1 },

    // ── f32 numeric instructions ──────────────────────────────────────────────
    //
    // IEEE 754 single-precision (32-bit) floating-point instructions.
    //
    // Comparison results are i32 (1 = true, 0 = false), just like integer
    // comparisons.  NaN comparisons always return 0 (false).
    //
    // Unary operations: abs, neg, ceil, floor, trunc, nearest, sqrt.
    // Binary operations: add, sub, mul, div, min, max, copysign.
    //
    // `f32.nearest` rounds to the nearest integer, ties to even (banker's rounding).
    // `f32.copysign` copies the sign bit from the second operand to the first.
    OpcodeInfo { name: "f32.const",   opcode: 0x43, category: "numeric_f32", immediates: &["f32"], stack_pop: 0, stack_push: 1 },
    OpcodeInfo { name: "f32.eq",      opcode: 0x5B, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.ne",      opcode: 0x5C, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.lt",      opcode: 0x5D, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.gt",      opcode: 0x5E, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.le",      opcode: 0x5F, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.ge",      opcode: 0x60, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.abs",     opcode: 0x8B, category: "numeric_f32", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.neg",     opcode: 0x8C, category: "numeric_f32", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.ceil",    opcode: 0x8D, category: "numeric_f32", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.floor",   opcode: 0x8E, category: "numeric_f32", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.trunc",   opcode: 0x8F, category: "numeric_f32", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.nearest", opcode: 0x90, category: "numeric_f32", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.sqrt",    opcode: 0x91, category: "numeric_f32", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.add",     opcode: 0x92, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.sub",     opcode: 0x93, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.mul",     opcode: 0x94, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.div",     opcode: 0x95, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.min",     opcode: 0x96, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.max",     opcode: 0x97, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f32.copysign",opcode: 0x98, category: "numeric_f32", immediates: &[],      stack_pop: 2, stack_push: 1 },

    // ── f64 numeric instructions ──────────────────────────────────────────────
    //
    // IEEE 754 double-precision (64-bit) floating-point. Mirror of f32 set.
    OpcodeInfo { name: "f64.const",   opcode: 0x44, category: "numeric_f64", immediates: &["f64"], stack_pop: 0, stack_push: 1 },
    OpcodeInfo { name: "f64.eq",      opcode: 0x61, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.ne",      opcode: 0x62, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.lt",      opcode: 0x63, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.gt",      opcode: 0x64, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.le",      opcode: 0x65, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.ge",      opcode: 0x66, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.abs",     opcode: 0x99, category: "numeric_f64", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.neg",     opcode: 0x9A, category: "numeric_f64", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.ceil",    opcode: 0x9B, category: "numeric_f64", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.floor",   opcode: 0x9C, category: "numeric_f64", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.trunc",   opcode: 0x9D, category: "numeric_f64", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.nearest", opcode: 0x9E, category: "numeric_f64", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.sqrt",    opcode: 0x9F, category: "numeric_f64", immediates: &[],      stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.add",     opcode: 0xA0, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.sub",     opcode: 0xA1, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.mul",     opcode: 0xA2, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.div",     opcode: 0xA3, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.min",     opcode: 0xA4, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.max",     opcode: 0xA5, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },
    OpcodeInfo { name: "f64.copysign",opcode: 0xA6, category: "numeric_f64", immediates: &[],      stack_pop: 2, stack_push: 1 },

    // ── Conversion instructions ───────────────────────────────────────────────
    //
    // Conversions change the type of a value on the stack.  All are unary
    // (pop one, push one).  The naming pattern is:
    //
    //   <dest_type>.<operation>_<source_type>
    //
    // Operations:
    //   `wrap`     — truncate a wider int to a narrower one (i64→i32, no data check)
    //   `extend`   — widen an integer, with explicit signedness (_s or _u)
    //   `trunc`    — convert float → int by truncating toward zero (can trap on NaN/inf)
    //   `convert`  — convert int → float
    //   `demote`   — narrow float (f64→f32), may lose precision
    //   `promote`  — widen float  (f32→f64), exact
    //   `reinterpret` — reinterpret the bits with no arithmetic conversion
    //                   (same bit pattern, different type)
    //
    // Reinterpret examples:
    //   i32.reinterpret_f32: treats the 4 bytes of an f32 as an i32 bit pattern
    //   f32.reinterpret_i32: the reverse
    OpcodeInfo { name: "i32.wrap_i64",      opcode: 0xA7, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.trunc_f32_s",   opcode: 0xA8, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.trunc_f32_u",   opcode: 0xA9, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.trunc_f64_s",   opcode: 0xAA, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.trunc_f64_u",   opcode: 0xAB, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.extend_i32_s",  opcode: 0xAC, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.extend_i32_u",  opcode: 0xAD, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.trunc_f32_s",   opcode: 0xAE, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.trunc_f32_u",   opcode: 0xAF, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.trunc_f64_s",   opcode: 0xB0, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.trunc_f64_u",   opcode: 0xB1, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.convert_i32_s", opcode: 0xB2, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.convert_i32_u", opcode: 0xB3, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.convert_i64_s", opcode: 0xB4, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.convert_i64_u", opcode: 0xB5, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.demote_f64",    opcode: 0xB6, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.convert_i32_s", opcode: 0xB7, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.convert_i32_u", opcode: 0xB8, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.convert_i64_s", opcode: 0xB9, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.convert_i64_u", opcode: 0xBA, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.promote_f32",   opcode: 0xBB, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.reinterpret_f32",opcode: 0xBC, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.reinterpret_f64",opcode: 0xBD, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f32.reinterpret_i32",opcode: 0xBE, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "f64.reinterpret_i64",opcode: 0xBF, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },

    // ── Sign-extension instructions ───────────────────────────────────────────
    //
    // Added by the "sign-extension operators" proposal (widely implemented,
    // MVP-adjacent) — still single-byte, unlike the later proposals' 0xFC-
    // prefixed opcodes (see this crate's own module doc comment). Each takes
    // the LOW N bits of the operand and sign-extends them to fill the full
    // i32/i64 width, treating those low bits as two's-complement signed —
    // e.g. `i32.extend8_s` on 0xFF (255) produces -1 (0xFFFFFFFF), the same
    // value `i32.load8_s` would produce loading that byte from memory.
    OpcodeInfo { name: "i32.extend8_s",  opcode: 0xC0, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i32.extend16_s", opcode: 0xC1, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.extend8_s",  opcode: 0xC2, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.extend16_s", opcode: 0xC3, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },
    OpcodeInfo { name: "i64.extend32_s", opcode: 0xC4, category: "conversion", immediates: &[], stack_pop: 1, stack_push: 1 },

    // ── `ref.func` (reference-types proposal, WASM17) ─────────────────────────
    //
    // Pushes a `funcref` referring to a function by index (used to obtain a
    // callable reference without going through a table, e.g. as an
    // `elem`-segment initializer or a first-class value). Takes one
    // `funcidx` LEB128 immediate, same shape as `call`'s immediate.
    //
    // `ref.null` (0xD0) and `ref.is_null` (0xD1) are deliberately absent from
    // this table: they already have working handlers in `wasm-execution`
    // and `wasm-validator` from before WASM17, and `wasm-execution`'s
    // decoder special-cases `ref.null`'s heap-type immediate outside this
    // generic table (see `code/specs/W08-wasm-funcref-externref.md`).
    OpcodeInfo { name: "ref.func", opcode: 0xD2, category: "reference", immediates: &["funcidx"], stack_pop: 0, stack_push: 1 },
];

// ──────────────────────────────────────────────────────────────────────────────
// Atomic memory operations (0xFE prefix, threads proposal — WASM18)
// ──────────────────────────────────────────────────────────────────────────────
//
// Like `0xFB`/`0xFC`, `0xFE` is a two-byte prefix encoding
// (`0xFE <sub-opcode> ...`) that doesn't fit this crate's single-byte
// `OPCODES` table, so these entries live in a SEPARATE table, keyed by
// the sub-opcode byte (the byte AFTER `0xFE`), not folded into `OPCODES`
// itself. Unlike `0xFB`/`0xFC` (whose sub-opcode dispatch is duplicated
// ad hoc in each consumer, since this repo's slice of those prefixes is
// small and irregular), the atomic family is regular enough — 64
// opcodes across a handful of repeating shapes — that centralizing the
// name/value-type/width table here, as the one shared source of truth
// `wasm-wast-parser`/`wasm-execution`/`wasm-validator` all key off,
// avoids tripling up the same 31-row table three times. See
// `code/specs/W09-wasm-atomics-plain.md`.

/// What shape of stack effect an atomic opcode has -- see each variant's
/// own doc comment for the exact pop/push list a consumer should apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicOpKind {
    /// Pops an `i32` address, pushes the loaded value.
    Load,
    /// Pops an `i32` address and a value, pushes nothing.
    Store,
    /// Pops an `i32` address and a value, pushes the value that was
    /// there BEFORE the read-modify-write (the spec's own semantics —
    /// every RMW op returns the OLD value, not the new one).
    Rmw,
    /// Pops an `i32` address, an expected value, and a replacement
    /// value; pushes whatever was actually there before the (possibly
    /// no-op) exchange -- same "always returns the old value" shape as
    /// `Rmw`, just with two value operands instead of one.
    Cmpxchg,
    /// No operands, no stack effect at all (`atomic.fence` — a true
    /// no-op with a single native thread, see the spec's own "Why" for
    /// the reasoning).
    Fence,
    /// `memory.atomic.notify`: pops an `i32` address and an `i32` waiter
    /// count, pushes an `i32` count of how many waiters were actually
    /// woken. With one native thread there is never a second agent
    /// blocked in `wait`, so this always resolves to `0` — a real,
    /// deterministic answer, not a stand-in for unimplemented behavior.
    /// `value_type` is `None` (the waiter count isn't the memory's own
    /// value type).
    Notify,
    /// `memory.atomic.wait32`/`wait64`: pops an `i32` address, an
    /// expected value (`value_type` — `I32` for `wait32`, `I64` for
    /// `wait64`), and an `i64` timeout; pushes an `i32` result code.
    /// With one native thread there is never a second agent able to
    /// `notify` this wait, so the only two REAL spec outcomes reachable
    /// here are `1` ("not-equal", when the current memory value already
    /// doesn't match `expected`) and `2` ("timed-out", when it does) —
    /// `0` ("ok", woken by a real `notify`) can never happen. Confirmed
    /// against the real, pinned-commit testsuite's own `wait32`/`wait64`
    /// assertions, which exercise exactly the "not-equal" path.
    Wait,
}

/// One entry in the atomic opcode table: everything a consumer needs to
/// decode, type-check, and execute one `0xFE`-prefixed instruction.
#[derive(Debug, Clone, Copy)]
pub struct AtomicOpInfo {
    /// The canonical text name, e.g. `"i32.atomic.load"`.
    pub name: &'static str,
    /// The sub-opcode byte (the byte immediately after the `0xFE` prefix).
    pub sub_opcode: u8,
    pub kind: AtomicOpKind,
    /// The `i32`/`i64` value type this op loads/stores/reads/writes.
    /// `None` for `Fence`, which touches no value at all.
    pub value_type: Option<wasm_types::ValueType>,
    /// The REQUIRED alignment in bytes (1, 2, 4, or 8) -- unlike plain
    /// loads/stores (whose `align` immediate is only an upper-bound
    /// hint), atomic accesses must be naturally aligned exactly. `0` for
    /// `Fence`, which has no memory access to align.
    pub natural_align: u32,
}

/// All 64 atomic opcodes: the plain load/store/RMW/cmpxchg/fence family
/// `code/specs/W09-wasm-atomics-plain.md` designed, PLUS
/// `memory.atomic.notify`/`wait32`/`wait64` (sub-opcodes `0x00`-`0x02`).
/// That spec's own prose claims notify/wait are "deliberately absent --
/// meaningless without real threads"; implementation-time verification
/// against the real, pinned-commit `proposals/threads/atomic.wast`
/// testsuite file found otherwise -- notify/wait DO have well-defined,
/// fully deterministic single-agent semantics (see `AtomicOpKind::
/// Notify`/`Wait`'s own doc comments), and the vendored file's own
/// assertions exercise exactly that deterministic path, not anything
/// requiring a second real thread. Implemented for real here rather than
/// left as a stub trap.
pub static ATOMIC_OPS: &[AtomicOpInfo] = &[
    // ── Notify/wait (WASM18) ───────────────────────────────────────────
    AtomicOpInfo { name: "memory.atomic.notify", sub_opcode: 0x00, kind: AtomicOpKind::Notify, value_type: None, natural_align: 4 },
    AtomicOpInfo { name: "memory.atomic.wait32", sub_opcode: 0x01, kind: AtomicOpKind::Wait, value_type: Some(wasm_types::ValueType::I32), natural_align: 4 },
    AtomicOpInfo { name: "memory.atomic.wait64", sub_opcode: 0x02, kind: AtomicOpKind::Wait, value_type: Some(wasm_types::ValueType::I64), natural_align: 8 },
    AtomicOpInfo { name: "atomic.fence", sub_opcode: 0x03, kind: AtomicOpKind::Fence, value_type: None, natural_align: 0 },
    // ── Loads ──────────────────────────────────────────────────────────
    AtomicOpInfo { name: "i32.atomic.load", sub_opcode: 0x10, kind: AtomicOpKind::Load, value_type: Some(wasm_types::ValueType::I32), natural_align: 4 },
    AtomicOpInfo { name: "i64.atomic.load", sub_opcode: 0x11, kind: AtomicOpKind::Load, value_type: Some(wasm_types::ValueType::I64), natural_align: 8 },
    AtomicOpInfo { name: "i32.atomic.load8_u", sub_opcode: 0x12, kind: AtomicOpKind::Load, value_type: Some(wasm_types::ValueType::I32), natural_align: 1 },
    AtomicOpInfo { name: "i32.atomic.load16_u", sub_opcode: 0x13, kind: AtomicOpKind::Load, value_type: Some(wasm_types::ValueType::I32), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.load8_u", sub_opcode: 0x14, kind: AtomicOpKind::Load, value_type: Some(wasm_types::ValueType::I64), natural_align: 1 },
    AtomicOpInfo { name: "i64.atomic.load16_u", sub_opcode: 0x15, kind: AtomicOpKind::Load, value_type: Some(wasm_types::ValueType::I64), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.load32_u", sub_opcode: 0x16, kind: AtomicOpKind::Load, value_type: Some(wasm_types::ValueType::I64), natural_align: 4 },
    // ── Stores ─────────────────────────────────────────────────────────
    AtomicOpInfo { name: "i32.atomic.store", sub_opcode: 0x17, kind: AtomicOpKind::Store, value_type: Some(wasm_types::ValueType::I32), natural_align: 4 },
    AtomicOpInfo { name: "i64.atomic.store", sub_opcode: 0x18, kind: AtomicOpKind::Store, value_type: Some(wasm_types::ValueType::I64), natural_align: 8 },
    AtomicOpInfo { name: "i32.atomic.store8", sub_opcode: 0x19, kind: AtomicOpKind::Store, value_type: Some(wasm_types::ValueType::I32), natural_align: 1 },
    AtomicOpInfo { name: "i32.atomic.store16", sub_opcode: 0x1A, kind: AtomicOpKind::Store, value_type: Some(wasm_types::ValueType::I32), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.store8", sub_opcode: 0x1B, kind: AtomicOpKind::Store, value_type: Some(wasm_types::ValueType::I64), natural_align: 1 },
    AtomicOpInfo { name: "i64.atomic.store16", sub_opcode: 0x1C, kind: AtomicOpKind::Store, value_type: Some(wasm_types::ValueType::I64), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.store32", sub_opcode: 0x1D, kind: AtomicOpKind::Store, value_type: Some(wasm_types::ValueType::I64), natural_align: 4 },
    // ── RMW: add (0x1E-0x24), sub (0x25-0x2B), and (0x2C-0x32), or
    // (0x33-0x39), xor (0x3A-0x40), xchg (0x41-0x47) -- each a 7-slot
    // block ordered i32, i64, i32-8, i32-16, i64-8, i64-16, i64-32. ─────
    AtomicOpInfo { name: "i32.atomic.rmw.add", sub_opcode: 0x1E, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 4 },
    AtomicOpInfo { name: "i64.atomic.rmw.add", sub_opcode: 0x1F, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 8 },
    AtomicOpInfo { name: "i32.atomic.rmw8.add_u", sub_opcode: 0x20, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 1 },
    AtomicOpInfo { name: "i32.atomic.rmw16.add_u", sub_opcode: 0x21, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw8.add_u", sub_opcode: 0x22, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 1 },
    AtomicOpInfo { name: "i64.atomic.rmw16.add_u", sub_opcode: 0x23, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw32.add_u", sub_opcode: 0x24, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 4 },

    AtomicOpInfo { name: "i32.atomic.rmw.sub", sub_opcode: 0x25, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 4 },
    AtomicOpInfo { name: "i64.atomic.rmw.sub", sub_opcode: 0x26, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 8 },
    AtomicOpInfo { name: "i32.atomic.rmw8.sub_u", sub_opcode: 0x27, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 1 },
    AtomicOpInfo { name: "i32.atomic.rmw16.sub_u", sub_opcode: 0x28, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw8.sub_u", sub_opcode: 0x29, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 1 },
    AtomicOpInfo { name: "i64.atomic.rmw16.sub_u", sub_opcode: 0x2A, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw32.sub_u", sub_opcode: 0x2B, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 4 },

    AtomicOpInfo { name: "i32.atomic.rmw.and", sub_opcode: 0x2C, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 4 },
    AtomicOpInfo { name: "i64.atomic.rmw.and", sub_opcode: 0x2D, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 8 },
    AtomicOpInfo { name: "i32.atomic.rmw8.and_u", sub_opcode: 0x2E, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 1 },
    AtomicOpInfo { name: "i32.atomic.rmw16.and_u", sub_opcode: 0x2F, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw8.and_u", sub_opcode: 0x30, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 1 },
    AtomicOpInfo { name: "i64.atomic.rmw16.and_u", sub_opcode: 0x31, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw32.and_u", sub_opcode: 0x32, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 4 },

    AtomicOpInfo { name: "i32.atomic.rmw.or", sub_opcode: 0x33, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 4 },
    AtomicOpInfo { name: "i64.atomic.rmw.or", sub_opcode: 0x34, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 8 },
    AtomicOpInfo { name: "i32.atomic.rmw8.or_u", sub_opcode: 0x35, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 1 },
    AtomicOpInfo { name: "i32.atomic.rmw16.or_u", sub_opcode: 0x36, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw8.or_u", sub_opcode: 0x37, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 1 },
    AtomicOpInfo { name: "i64.atomic.rmw16.or_u", sub_opcode: 0x38, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw32.or_u", sub_opcode: 0x39, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 4 },

    AtomicOpInfo { name: "i32.atomic.rmw.xor", sub_opcode: 0x3A, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 4 },
    AtomicOpInfo { name: "i64.atomic.rmw.xor", sub_opcode: 0x3B, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 8 },
    AtomicOpInfo { name: "i32.atomic.rmw8.xor_u", sub_opcode: 0x3C, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 1 },
    AtomicOpInfo { name: "i32.atomic.rmw16.xor_u", sub_opcode: 0x3D, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw8.xor_u", sub_opcode: 0x3E, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 1 },
    AtomicOpInfo { name: "i64.atomic.rmw16.xor_u", sub_opcode: 0x3F, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw32.xor_u", sub_opcode: 0x40, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 4 },

    AtomicOpInfo { name: "i32.atomic.rmw.xchg", sub_opcode: 0x41, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 4 },
    AtomicOpInfo { name: "i64.atomic.rmw.xchg", sub_opcode: 0x42, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 8 },
    AtomicOpInfo { name: "i32.atomic.rmw8.xchg_u", sub_opcode: 0x43, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 1 },
    AtomicOpInfo { name: "i32.atomic.rmw16.xchg_u", sub_opcode: 0x44, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I32), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw8.xchg_u", sub_opcode: 0x45, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 1 },
    AtomicOpInfo { name: "i64.atomic.rmw16.xchg_u", sub_opcode: 0x46, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw32.xchg_u", sub_opcode: 0x47, kind: AtomicOpKind::Rmw, value_type: Some(wasm_types::ValueType::I64), natural_align: 4 },
    // ── Cmpxchg (0x48-0x4E), same 7-slot shape, two value operands ────
    AtomicOpInfo { name: "i32.atomic.rmw.cmpxchg", sub_opcode: 0x48, kind: AtomicOpKind::Cmpxchg, value_type: Some(wasm_types::ValueType::I32), natural_align: 4 },
    AtomicOpInfo { name: "i64.atomic.rmw.cmpxchg", sub_opcode: 0x49, kind: AtomicOpKind::Cmpxchg, value_type: Some(wasm_types::ValueType::I64), natural_align: 8 },
    AtomicOpInfo { name: "i32.atomic.rmw8.cmpxchg_u", sub_opcode: 0x4A, kind: AtomicOpKind::Cmpxchg, value_type: Some(wasm_types::ValueType::I32), natural_align: 1 },
    AtomicOpInfo { name: "i32.atomic.rmw16.cmpxchg_u", sub_opcode: 0x4B, kind: AtomicOpKind::Cmpxchg, value_type: Some(wasm_types::ValueType::I32), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw8.cmpxchg_u", sub_opcode: 0x4C, kind: AtomicOpKind::Cmpxchg, value_type: Some(wasm_types::ValueType::I64), natural_align: 1 },
    AtomicOpInfo { name: "i64.atomic.rmw16.cmpxchg_u", sub_opcode: 0x4D, kind: AtomicOpKind::Cmpxchg, value_type: Some(wasm_types::ValueType::I64), natural_align: 2 },
    AtomicOpInfo { name: "i64.atomic.rmw32.cmpxchg_u", sub_opcode: 0x4E, kind: AtomicOpKind::Cmpxchg, value_type: Some(wasm_types::ValueType::I64), natural_align: 4 },
];

/// Look up an atomic opcode by its sub-opcode byte (the byte after the
/// `0xFE` prefix).
///
/// # Example
///
/// ```
/// use wasm_opcodes::get_atomic_op;
///
/// let info = get_atomic_op(0x10).unwrap();
/// assert_eq!(info.name, "i32.atomic.load");
/// ```
pub fn get_atomic_op(sub_opcode: u8) -> Option<&'static AtomicOpInfo> {
    ATOMIC_OPS.iter().find(|op| op.sub_opcode == sub_opcode)
}

/// Look up an atomic opcode by its canonical text name, e.g.
/// `"i32.atomic.load"`, `"i64.atomic.rmw.cmpxchg"`.
///
/// # Example
///
/// ```
/// use wasm_opcodes::get_atomic_op_by_name;
///
/// let info = get_atomic_op_by_name("i32.atomic.load").unwrap();
/// assert_eq!(info.sub_opcode, 0x10);
/// ```
pub fn get_atomic_op_by_name(name: &str) -> Option<&'static AtomicOpInfo> {
    ATOMIC_OPS.iter().find(|op| op.name == name)
}

// ──────────────────────────────────────────────────────────────────────────────
// SIMD (v128) operations (0xFD prefix -- see code/specs/
// W13-wasm-simd-v128-first-slice.md)
// ──────────────────────────────────────────────────────────────────────────────
//
// Like `0xFB`/`0xFC`/`0xFE`, `0xFD` is a two-byte-PREFIX encoding
// (`0xFD <sub-opcode> ...`) that doesn't fit this crate's single-byte
// `OPCODES` table. It DOESN'T fit the `AtomicOpInfo`/`ATOMIC_OPS` shape
// either, though: SIMD's sub-opcode is a **LEB128-encoded `u32`**, not a
// raw byte -- confirmed against the SIMD proposal's own binary-encoding
// table (`BinarySIMD.md`) and the W3C core spec's "Vector Instructions"
// section, both independently. `i32x4.add`'s real sub-opcode is `174`
// (`0xAE`), which needs the two-byte LEB128 continuation encoding
// (`[0xAE, 0x01]`) -- a `u8` field, as `AtomicOpInfo::sub_opcode` uses,
// cannot represent it at all.

/// What this `SimdOpInfo` entry does at execution time. This intentionally
/// covers only the `i32x4` lane width plus `v128.const`; expect this to
/// grow real shape (other lane widths, more arithmetic ops, shuffles,
/// etc.) as later PRs add more of the ~230-opcode family -- see
/// `code/specs/W13-wasm-simd-v128-first-slice.md`'s staged-PR plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdOpKind {
    /// `v128.const` -- push a 16-byte immediate constant.
    Const,
    /// `i32x4.splat` -- broadcast one `i32` into all 4 lanes.
    Splat,
    /// `i32x4.add` -- lane-wise wrapping addition.
    Add,
    /// `i32x4.eq` -- lane-wise equality; each lane becomes all-1s (`-1`)
    /// if equal, all-0s (`0`) otherwise (WASM's boolean-mask convention
    /// for SIMD comparisons).
    Eq,
    /// `i32x4.ne` -- lane-wise inequality, same boolean-mask convention as `Eq`.
    Ne,
    /// `i32x4.lt_s` -- lane-wise signed less-than, boolean-mask convention.
    LtS,
    /// `i32x4.lt_u` -- lane-wise unsigned less-than, boolean-mask convention.
    LtU,
    /// `i32x4.gt_s` -- lane-wise signed greater-than, boolean-mask convention.
    GtS,
    /// `i32x4.gt_u` -- lane-wise unsigned greater-than, boolean-mask convention.
    GtU,
    /// `i32x4.le_s` -- lane-wise signed less-than-or-equal, boolean-mask convention.
    LeS,
    /// `i32x4.le_u` -- lane-wise unsigned less-than-or-equal, boolean-mask convention.
    LeU,
    /// `i32x4.ge_s` -- lane-wise signed greater-than-or-equal, boolean-mask convention.
    GeS,
    /// `i32x4.ge_u` -- lane-wise unsigned greater-than-or-equal, boolean-mask convention.
    GeU,
    /// `i32x4.sub` -- lane-wise wrapping subtraction.
    Sub,
    /// `i32x4.mul` -- lane-wise wrapping multiplication.
    Mul,
    /// `i32x4.neg` -- lane-wise arithmetic negation. UNARY, unlike every
    /// other kind above (`Add`/`Sub`/`Mul`/`Eq`/`Ne`/comparisons all pop
    /// TWO `v128`s and push one; `Neg` pops exactly ONE `v128` and pushes
    /// one) -- see this module's own execution-handler dispatch, which
    /// funnels `Neg` through a separate arm from the binary lane-wise ops.
    Neg,
    /// `i32x4.abs` -- lane-wise absolute value. UNARY, same shape as `Neg`.
    Abs,
    /// `i32x4.min_s` -- lane-wise signed minimum.
    MinS,
    /// `i32x4.min_u` -- lane-wise unsigned minimum.
    MinU,
    /// `i32x4.max_s` -- lane-wise signed maximum.
    MaxS,
    /// `i32x4.max_u` -- lane-wise unsigned maximum.
    MaxU,
    /// `i32x4.extract_lane` -- read one `i32` lane back out of a `v128`,
    /// selected by a lane-index immediate (0-3). Not in the spec's
    /// original 4-opcode minimal slice, but genuinely required to make
    /// this first slice's OWN correctness verifiable at all: without
    /// some way to observe a `v128`'s contents as a plain scalar, no
    /// integration test (or, more importantly, no `wasm-conformance`
    /// `assert_return` grading) could distinguish a correct SIMD
    /// computation from a subtly wrong one.
    ExtractLane,
    /// `i32x4.extadd_pairwise_i16x8_s` -- reinterpret the operand `v128`
    /// as 8 SIGNED `i16` lanes, pairwise-add adjacent lanes (0+1, 2+3,
    /// 4+5, 6+7) with each addend sign-extended to `i32` first, producing
    /// a 4-lane `i32x4` result. UNARY (pop one `v128`, push one), like
    /// `Neg`/`Abs` -- but unlike them, the INPUT lane width (16-bit) and
    /// OUTPUT lane width (32-bit) differ, the first opcode in this crate
    /// where that's true.
    ExtaddPairwiseI16x8S,
    /// `i32x4.extadd_pairwise_i16x8_u` -- same pairwise-add shape as
    /// [`Self::ExtaddPairwiseI16x8S`], but each `i16` lane is
    /// zero-extended (read as `u16`) before the add, not sign-extended.
    ExtaddPairwiseI16x8U,
    /// `i32x4.dot_i16x8_s` -- reinterpret BOTH `v128` operands as 8
    /// SIGNED `i16` lanes each; for each of the 4 result lanes `i`,
    /// compute `sext(a[2i]) * sext(b[2i]) + sext(a[2i+1]) * sext(b[2i+1])`
    /// (a per-pair signed multiply-accumulate), producing an `i32x4`
    /// result. BINARY (pop two `v128`s, push one), but -- like
    /// [`Self::ExtaddPairwiseI16x8S`] -- the input lane width (16-bit)
    /// differs from the output lane width (32-bit), unlike every prior
    /// binary kind in this enum.
    DotI16x8S,
    /// `i32x4.extmul_low_i16x8_s` -- reinterpret both `v128` operands as 8
    /// SIGNED `i16` lanes each; take only the LOW 4 lanes (indices 0-3) of
    /// each, sign-extend every value to `i32`, and multiply the
    /// corresponding pairs lane-wise, producing an `i32x4` result. Same
    /// narrow-input/wide-output BINARY shape as [`Self::DotI16x8S`], but
    /// without the pairwise summation -- a plain widening multiply.
    ExtmulLowI16x8S,
    /// `i32x4.extmul_high_i16x8_s` -- same as
    /// [`Self::ExtmulLowI16x8S`], but operates on the HIGH 4 lanes
    /// (indices 4-7) of each `i16x8` operand instead of the low 4.
    ExtmulHighI16x8S,
    /// `i32x4.extmul_low_i16x8_u` -- same LOW-4-lanes widening multiply as
    /// [`Self::ExtmulLowI16x8S`], but each `i16` lane is zero-extended
    /// (read as `u16`) before the multiply, not sign-extended.
    ExtmulLowI16x8U,
    /// `i32x4.extmul_high_i16x8_u` -- same HIGH-4-lanes widening multiply
    /// as [`Self::ExtmulHighI16x8S`], but zero-extended, not sign-extended.
    ExtmulHighI16x8U,
    /// `i8x16.add` -- lane-wise wrapping addition over 16 `i8` lanes. The
    /// first opcode in this table for the `i8x16` lane width -- unlike
    /// `i32x4`'s first slice, no `i8x16.splat`/`extract_lane` are needed
    /// alongside it, since `v128.const i8x16 ...` (already supported for
    /// all 6 shapes) covers both operand construction and result
    /// comparison for this first `i8x16` slice's own test corpus.
    AddI8x16,
    /// `i8x16.sub` -- lane-wise wrapping subtraction over 16 `i8` lanes.
    SubI8x16,
    /// `i8x16.neg` -- lane-wise arithmetic negation over 16 `i8` lanes.
    /// UNARY, same shape as `i32x4`'s own `Neg`/`Abs`. WASM SIMD defines
    /// no `i8x16.mul` (8-bit lanes are too narrow for a useful lane-wise
    /// multiply in this proposal), so this first `i8x16` slice is
    /// deliberately just these 3 opcodes, not a 4th "mul" entry.
    NegI8x16,
    /// `i16x8.add` -- lane-wise wrapping addition over 8 `i16` lanes. The
    /// first opcode in this table for `i16x8` as a PRIMARY lane width
    /// (this table already has opcodes that READ `i16x8` operands --
    /// `ExtaddPairwiseI16x8S`/`DotI16x8S`/etc. -- but those all WRITE an
    /// `i32x4` result; this is the first opcode whose result is itself
    /// `i16x8`). Same "first slice" pattern as `i8x16.add`/`sub`/`neg`:
    /// no `i16x8.splat`/`extract_lane` needed, since `v128.const i16x8
    /// ...` (already supported for all 6 shapes) covers this slice's own
    /// operand construction and result comparison on its own.
    AddI16x8,
    /// `i16x8.sub` -- lane-wise wrapping subtraction over 8 `i16` lanes.
    SubI16x8,
    /// `i16x8.mul` -- lane-wise wrapping multiplication over 8 `i16`
    /// lanes. Unlike `i8x16` (whose 8-bit lanes are too narrow for a
    /// useful lane-wise multiply, so the spec defines none), WASM SIMD
    /// DOES define `i16x8.mul` -- this slice includes it precisely
    /// because the real upstream corpus file (`simd_i16x8_arith.wast`)
    /// bundles all four ops (`neg`/`add`/`sub`/`mul`) together.
    MulI16x8,
    /// `i16x8.neg` -- lane-wise arithmetic negation over 8 `i16` lanes.
    /// UNARY, same shape as `i8x16.neg`/`i32x4.neg`/`.abs`.
    NegI16x8,
    /// `i16x8.eq` -- lane-wise equality over 8 `i16` lanes; each lane
    /// becomes all-1s if equal, all-0s otherwise (same boolean-mask
    /// convention as `Eq`/`Ne`/etc. above, but for `i16x8` -- the first
    /// comparison family for a lane width other than `i32x4`).
    EqI16x8,
    /// `i16x8.ne` -- lane-wise inequality, same boolean-mask convention as
    /// [`Self::EqI16x8`].
    NeI16x8,
    /// `i16x8.lt_s` -- lane-wise signed less-than, boolean-mask convention.
    LtSI16x8,
    /// `i16x8.lt_u` -- lane-wise unsigned less-than, boolean-mask convention.
    LtUI16x8,
    /// `i16x8.gt_s` -- lane-wise signed greater-than, boolean-mask convention.
    GtSI16x8,
    /// `i16x8.gt_u` -- lane-wise unsigned greater-than, boolean-mask convention.
    GtUI16x8,
    /// `i16x8.le_s` -- lane-wise signed less-than-or-equal, boolean-mask convention.
    LeSI16x8,
    /// `i16x8.le_u` -- lane-wise unsigned less-than-or-equal, boolean-mask convention.
    LeUI16x8,
    /// `i16x8.ge_s` -- lane-wise signed greater-than-or-equal, boolean-mask convention.
    GeSI16x8,
    /// `i16x8.ge_u` -- lane-wise unsigned greater-than-or-equal, boolean-mask convention.
    GeUI16x8,
    /// `i8x16.eq` -- lane-wise equality over 16 `i8` lanes; each lane
    /// becomes all-1s if equal, all-0s otherwise (same boolean-mask
    /// convention as `i16x8`'s own comparison family, closing the same
    /// gap for `i8x16`, which had arith but no comparison family until
    /// now).
    EqI8x16,
    /// `i8x16.ne` -- lane-wise inequality, same boolean-mask convention as
    /// [`Self::EqI8x16`].
    NeI8x16,
    /// `i8x16.lt_s` -- lane-wise signed less-than, boolean-mask convention.
    LtSI8x16,
    /// `i8x16.lt_u` -- lane-wise unsigned less-than, boolean-mask convention.
    LtUI8x16,
    /// `i8x16.gt_s` -- lane-wise signed greater-than, boolean-mask convention.
    GtSI8x16,
    /// `i8x16.gt_u` -- lane-wise unsigned greater-than, boolean-mask convention.
    GtUI8x16,
    /// `i8x16.le_s` -- lane-wise signed less-than-or-equal, boolean-mask convention.
    LeSI8x16,
    /// `i8x16.le_u` -- lane-wise unsigned less-than-or-equal, boolean-mask convention.
    LeUI8x16,
    /// `i8x16.ge_s` -- lane-wise signed greater-than-or-equal, boolean-mask convention.
    GeSI8x16,
    /// `i8x16.ge_u` -- lane-wise unsigned greater-than-or-equal, boolean-mask convention.
    GeUI8x16,
    /// `i8x16.abs` -- lane-wise absolute value, wrapping (`i8::MIN.abs()`
    /// wraps back to `i8::MIN`, same two's-complement discipline as
    /// [`Self::Neg`]/[`Self::Abs`] at `i32x4`'s width).
    AbsI8x16,
    /// `i8x16.popcnt` -- lane-wise population count (Hamming weight) of
    /// each `i8` lane's bits. First SIMD popcnt in this table -- no
    /// `i32x4`/`i16x8` precedent, since WASM SIMD only defines `popcnt`
    /// for `i8x16`.
    PopcntI8x16,
    /// `i8x16.min_s` -- lane-wise signed minimum.
    MinSI8x16,
    /// `i8x16.min_u` -- lane-wise unsigned minimum.
    MinUI8x16,
    /// `i8x16.max_s` -- lane-wise signed maximum.
    MaxSI8x16,
    /// `i8x16.max_u` -- lane-wise unsigned maximum.
    MaxUI8x16,
    /// `i8x16.avgr_u` -- lane-wise unsigned rounding average:
    /// `(a + b + 1) >> 1`, computed widened (as `u16`) to avoid overflow.
    /// First SIMD avgr in this table -- no `i32x4`/`i16x8` precedent.
    AvgrUI8x16,
    /// `i16x8.abs` -- lane-wise absolute value, wrapping (`i16::MIN.abs()`
    /// wraps back to `i16::MIN`, same two's-complement discipline as
    /// [`Self::AbsI8x16`] at `i8x16`'s width).
    AbsI16x8,
    /// `i16x8.min_s` -- lane-wise signed minimum.
    MinSI16x8,
    /// `i16x8.min_u` -- lane-wise unsigned minimum.
    MinUI16x8,
    /// `i16x8.max_s` -- lane-wise signed maximum.
    MaxSI16x8,
    /// `i16x8.max_u` -- lane-wise unsigned maximum.
    MaxUI16x8,
    /// `i16x8.avgr_u` -- lane-wise unsigned rounding average:
    /// `(a + b + 1) >> 1`, computed widened (as `u32`) to avoid overflow.
    /// Same convention as [`Self::AvgrUI8x16`], just at `i16x8`'s width
    /// (WASM SIMD defines `avgr_u` for `i8x16`/`i16x8` but not `i32x4`;
    /// there is no `i16x8.popcnt` -- WASM SIMD only defines `popcnt` for
    /// `i8x16`).
    AvgrUI16x8,
    /// `i16x8.extadd_pairwise_i8x16_s` -- reinterpret the operand `v128` as
    /// 16 SIGNED `i8` lanes, pairwise-add adjacent lanes (0+1, 2+3, ...,
    /// 14+15) with each addend sign-extended to `i16` first, producing an
    /// 8-lane `i16x8` result. UNARY, narrow-input (8-bit)/wide-output
    /// (16-bit), same shape as [`Self::ExtaddPairwiseI16x8S`] one lane
    /// width down. Closes the last remaining gap between `i16x8` and
    /// `i8x16`'s coverage -- mirrors the already-implemented
    /// `i32x4`-from-`i16x8` widening family.
    ExtaddPairwiseI8x16S,
    /// `i16x8.extadd_pairwise_i8x16_u` -- same pairwise-add shape as
    /// [`Self::ExtaddPairwiseI8x16S`], but each `i8` lane is
    /// zero-extended (read as `u8`) before the add, not sign-extended.
    ExtaddPairwiseI8x16U,
    /// `i16x8.extmul_low_i8x16_s` -- reinterpret both `v128` operands as
    /// 16 SIGNED `i8` lanes each; take only the LOW 8 lanes (indices 0-7)
    /// of each, sign-extend every value to `i16`, and multiply the
    /// corresponding pairs lane-wise, producing an `i16x8` result. Same
    /// narrow-input/wide-output BINARY shape as
    /// [`Self::ExtmulLowI16x8S`] one lane width down. Unlike the
    /// `i32x4`-from-`i16x8` family, there is no `i16x8.dot_i8x16_s` --
    /// WASM SIMD does not define a dot-product for this pair, so this
    /// family has no `Dot*` counterpart.
    ExtmulLowI8x16S,
    /// `i16x8.extmul_high_i8x16_s` -- same as [`Self::ExtmulLowI8x16S`],
    /// but operates on the HIGH 8 lanes (indices 8-15) of each `i8x16`
    /// operand instead of the low 8.
    ExtmulHighI8x16S,
    /// `i16x8.extmul_low_i8x16_u` -- same LOW-8-lanes widening multiply as
    /// [`Self::ExtmulLowI8x16S`], but each `i8` lane is zero-extended
    /// (read as `u8`) before the multiply, not sign-extended.
    ExtmulLowI8x16U,
    /// `i16x8.extmul_high_i8x16_u` -- same HIGH-8-lanes widening multiply
    /// as [`Self::ExtmulHighI8x16S`], but zero-extended, not sign-extended.
    ExtmulHighI8x16U,
    /// `v128.not` -- bitwise NOT of all 128 bits, lane-width-agnostic (the
    /// result doesn't depend on how the bits are interpreted as lanes).
    /// UNARY, same shape as [`Self::Neg`]/[`Self::Abs`], but operates on
    /// the raw bytes rather than per-lane integers -- first bitwise op in
    /// this table, closing the gap between the narrow per-lane-width
    /// arithmetic families done so far and the far more universally-used
    /// masking/blending idioms every real SIMD program relies on.
    Not,
    /// `v128.and` -- bitwise AND of both operands' 128 bits,
    /// lane-width-agnostic. BINARY, same shape as [`Self::Add`]/
    /// [`Self::Sub`], but bytewise rather than per-lane.
    And,
    /// `v128.andnot` -- `a AND (NOT b)`, i.e. clears the bits in `a` that
    /// are set in `b`. BINARY, same shape as [`Self::And`].
    AndNot,
    /// `v128.or` -- bitwise OR of both operands' 128 bits,
    /// lane-width-agnostic. BINARY, same shape as [`Self::And`].
    Or,
    /// `v128.xor` -- bitwise XOR of both operands' 128 bits,
    /// lane-width-agnostic. BINARY, same shape as [`Self::And`].
    Xor,
    /// `v128.bitselect` -- ternary bitwise select: for each bit position,
    /// takes the bit from `a` where the corresponding bit of the mask `c`
    /// is `1`, otherwise the bit from `b` -- computed as
    /// `(a AND c) OR (b AND (NOT c))`. Pops THREE `v128`s, pushes one --
    /// the first ternary SIMD op in this interpreter, unlike every UNARY
    /// (pop one) or BINARY (pop two) kind above.
    Bitselect,
    /// `v128.any_true` -- pops one `v128`, pushes one `i32`: `1` if ANY of
    /// the 128 bits is set (equivalently, if any lane at any width is
    /// nonzero), else `0`. The first SCALAR-RESULT reduction kind in this
    /// table besides [`Self::ExtractLane`] -- unlike `ExtractLane`, there
    /// is no lane-index immediate, since this reduces over the WHOLE
    /// operand regardless of lane width.
    AnyTrue,
    /// `i8x16.all_true` -- pops one `v128`, pushes one `i32`: `1` if EVERY
    /// one of the 16 `i8` lanes is nonzero, else `0`. Same shape as
    /// [`Self::AnyTrue`] but ALL instead of ANY, and lane-width-sensitive
    /// (needs one variant per lane width, unlike `AnyTrue`).
    AllTrueI8x16,
    /// `i16x8.all_true` -- same as [`Self::AllTrueI8x16`], but over the 8
    /// `i16` lanes of an `i16x8`-interpreted operand.
    AllTrueI16x8,
    /// `i32x4.all_true` -- same as [`Self::AllTrueI8x16`], but over the 4
    /// `i32` lanes of an `i32x4`-interpreted operand.
    AllTrueI32x4,
    /// `i64x2.all_true` -- same as [`Self::AllTrueI8x16`], but over the 2
    /// `i64` lanes of an `i64x2`-interpreted operand -- the first opcode
    /// in this table to read the operand as 8-byte lanes.
    AllTrueI64x2,
    /// `i8x16.bitmask` -- pops one `v128`, pushes one `i32`: bit `i` of
    /// the result is the sign bit (MSB) of `i8` lane `i`, for all 16
    /// lanes, packed into the low 16 bits of the `i32`. Same
    /// v128-in/i32-out shape as [`Self::AnyTrue`]/[`Self::AllTrueI8x16`],
    /// but packs a per-lane BIT rather than reducing to a single
    /// true/false.
    BitmaskI8x16,
    /// `i16x8.bitmask` -- same as [`Self::BitmaskI8x16`], but one sign
    /// bit per `i16` lane (8 lanes), packed into the low 8 bits.
    BitmaskI16x8,
    /// `i32x4.bitmask` -- same as [`Self::BitmaskI8x16`], but one sign
    /// bit per `i32` lane (4 lanes), packed into the low 4 bits.
    BitmaskI32x4,
    /// `i64x2.bitmask` -- same as [`Self::BitmaskI8x16`], but one sign
    /// bit per `i64` lane (2 lanes), packed into the low 2 bits.
    BitmaskI64x2,
    /// `i64x2.abs` -- lane-wise absolute value of two SIGNED `i64` lanes,
    /// using `wrapping_abs` (so `i64::MIN` maps to itself, not a panic).
    /// UNARY, same shape as [`Self::AbsI8x16`] but at `i64x2`'s width --
    /// the first REAL ARITHMETIC opcode at this lane width (PR12 only
    /// added the all_true/bitmask reduction family).
    AbsI64x2,
    /// `i64x2.neg` -- lane-wise two's-complement negation of two `i64`
    /// lanes, using `wrapping_neg`. UNARY, same shape as
    /// [`Self::NegI8x16`] but at `i64x2`'s width.
    NegI64x2,
    /// `i64x2.add` -- lane-wise wrapping addition of two `i64x2`
    /// operands. BINARY, same shape as [`Self::Add`] but at `i64x2`'s
    /// width.
    AddI64x2,
    /// `i64x2.sub` -- lane-wise wrapping subtraction. BINARY, same shape
    /// as [`Self::AddI64x2`].
    SubI64x2,
    /// `i64x2.mul` -- lane-wise wrapping multiplication. BINARY, same
    /// shape as [`Self::AddI64x2`].
    MulI64x2,
    /// `i64x2.eq` -- lane-wise equality, boolean mask (all-1s/all-0s per
    /// `i64` lane). BINARY, same shape as [`Self::Eq`] but at `i64x2`'s
    /// width.
    EqI64x2,
    /// `i64x2.ne` -- lane-wise inequality, boolean mask. BINARY, same
    /// shape as [`Self::EqI64x2`].
    NeI64x2,
    /// `i64x2.lt_s` -- lane-wise SIGNED less-than, boolean mask. BINARY,
    /// same shape as [`Self::EqI64x2`]. No `lt_u` -- the SIMD proposal
    /// never defines unsigned `i64x2` comparisons, unlike every narrower
    /// lane width.
    LtSI64x2,
    /// `i64x2.gt_s` -- lane-wise SIGNED greater-than, boolean mask.
    /// BINARY, same shape as [`Self::EqI64x2`].
    GtSI64x2,
    /// `i64x2.le_s` -- lane-wise SIGNED less-than-or-equal, boolean mask.
    /// BINARY, same shape as [`Self::EqI64x2`].
    LeSI64x2,
    /// `i64x2.ge_s` -- lane-wise SIGNED greater-than-or-equal, boolean
    /// mask. BINARY, same shape as [`Self::EqI64x2`].
    GeSI64x2,
    /// `i8x16.shl` -- lane-wise logical shift LEFT by a SCALAR `i32`
    /// shift amount. The FIRST mixed-type binary op in this table: pops
    /// the `i32` shift amount (pushed LAST, so popped FIRST), then the
    /// `v128` operand, pushes one `v128` -- every prior binary op pops
    /// two `v128`s or one `v128`. Per the SIMD spec, the shift amount is
    /// taken MODULO the lane's bit width (8 here) before shifting --
    /// this is spec-mandated masking, not just a Rust safety necessity
    /// (shifting a Rust primitive by >= its bit width panics).
    ShlI8x16,
    /// `i8x16.shr_s` -- lane-wise ARITHMETIC (sign-extending) shift
    /// RIGHT by a scalar `i32` amount, masked modulo 8. Same
    /// mixed-type shape as [`Self::ShlI8x16`].
    ShrSI8x16,
    /// `i8x16.shr_u` -- lane-wise LOGICAL (zero-extending) shift RIGHT
    /// by a scalar `i32` amount, masked modulo 8. Same mixed-type shape
    /// as [`Self::ShlI8x16`]. Reinterprets each lane as `u8` before
    /// shifting so no sign bit is propagated in.
    ShrUI8x16,
    /// `i16x8.shl` -- same as [`Self::ShlI8x16`], but over 8 `i16`
    /// lanes, masked modulo 16.
    ShlI16x8,
    /// `i16x8.shr_s` -- same as [`Self::ShrSI8x16`], but over 8 `i16`
    /// lanes, masked modulo 16.
    ShrSI16x8,
    /// `i16x8.shr_u` -- same as [`Self::ShrUI8x16`], but over 8 `i16`
    /// lanes, masked modulo 16.
    ShrUI16x8,
    /// `i32x4.shl` -- same as [`Self::ShlI8x16`], but over 4 `i32`
    /// lanes, masked modulo 32.
    ShlI32x4,
    /// `i32x4.shr_s` -- same as [`Self::ShrSI8x16`], but over 4 `i32`
    /// lanes, masked modulo 32.
    ShrSI32x4,
    /// `i32x4.shr_u` -- same as [`Self::ShrUI8x16`], but over 4 `i32`
    /// lanes, masked modulo 32.
    ShrUI32x4,
    /// `i64x2.shl` -- same as [`Self::ShlI8x16`], but over 2 `i64`
    /// lanes, masked modulo 64.
    ShlI64x2,
    /// `i64x2.shr_s` -- same as [`Self::ShrSI8x16`], but over 2 `i64`
    /// lanes, masked modulo 64.
    ShrSI64x2,
    /// `i64x2.shr_u` -- same as [`Self::ShrUI8x16`], but over 2 `i64`
    /// lanes, masked modulo 64. Unlike `i64x2`'s comparison family
    /// (which has no unsigned variants), the shift family DOES define
    /// `shr_u` for `i64x2` -- shifting has no notion of "unsigned
    /// magnitude comparison" to omit.
    ShrUI64x2,
    /// `v128.load` -- read 16 bytes from linear memory at the effective
    /// address (`i32` base popped from the stack, plus this
    /// instruction's own `memarg` offset immediate), push a new `v128`.
    /// The FIRST SIMD load/store opcode in this table -- carries a
    /// `memarg` immediate (like every scalar `iNN.load`), not the
    /// 16-byte raw literal `Const` uses or the no-immediate shape most
    /// SIMD ops use. This first slice always targets memory 0 (no
    /// multi-memory support yet, unlike the scalar load/store family --
    /// see `code/specs/W13-wasm-simd-v128-first-slice.md` for the scope
    /// note).
    Load,
    /// `v128.store` -- pop a `v128`, pop an `i32` base address, write
    /// 16 bytes to linear memory at the effective address (base + this
    /// instruction's own `memarg` offset immediate). Same `memarg`
    /// shape and memory-0-only scope as [`Self::Load`].
    Store,
    /// `i8x16.splat` -- pop one `i32`, broadcast its LOW byte into all
    /// 16 lanes of a new `v128`. Same shape as [`Self::Splat`] (pop one
    /// scalar, push one `v128`) but at `i8x16`'s width -- only the
    /// low 8 bits of the popped `i32` matter, matching the spec's own
    /// "splat" semantics (the operand type is always `i32` for the
    /// integer splats narrower than `i64x2`, regardless of lane width).
    SplatI8x16,
    /// `i16x8.splat` -- pop one `i32`, broadcast its LOW 16 bits into
    /// all 8 lanes. Same shape as [`Self::SplatI8x16`], one lane width
    /// wider.
    SplatI16x8,
    /// `i64x2.splat` -- pop one `i64` (NOT `i32`, unlike every narrower
    /// integer splat), broadcast all 8 bytes into both lanes. Same
    /// "pop scalar, push v128" shape as [`Self::Splat`], but the FIRST
    /// splat whose popped operand type differs from `i32`.
    SplatI64x2,
    /// `f32x4.splat` -- pop one `f32`, broadcast its 4 little-endian
    /// bytes into all 4 lanes. The FIRST floating-point-typed SIMD
    /// instruction in this table: a pure bit-pattern broadcast, no
    /// rounding/NaN-canonicalization/comparison semantics, so it needs
    /// no new type-checker machinery beyond popping `F32` instead of
    /// `I32`/`I64`.
    SplatF32x4,
    /// `f64x2.splat` -- pop one `f64`, broadcast its 8 little-endian
    /// bytes into both lanes. Same shape as [`Self::SplatF32x4`], one
    /// lane width wider.
    SplatF64x2,
    /// `i8x16.swizzle` -- for each of the 16 result lanes `i`, look up
    /// `a[s[i]]` if `s[i] < 16`, else `0` -- a per-lane dynamic index
    /// (table lookup) into the FIRST operand `a` (the data vector) using
    /// byte indices read from the SECOND operand `s` (the index vector).
    /// Same "pop two v128s, push one" BINARY shape as `i8x16.add`/etc.,
    /// but the first SIMD op in this table whose per-lane computation
    /// uses the OTHER operand's bytes as an INDEX rather than a value to
    /// combine arithmetically/bitwise -- out-of-range indices (`>= 16`)
    /// produce `0`, not a trap or panic (verified against the SIMD
    /// proposal's own semantics, not an implementation guess -- see this
    /// package's own CHANGELOG entry for the PR that added it).
    Swizzle,
    /// `i8x16.extract_lane_s` -- read one `i8` lane back out of a `v128`,
    /// selected by a lane-index immediate (0-15, unlike
    /// [`Self::ExtractLane`]'s 0-3 range), SIGN-extended to `i32`. Same
    /// "pop v128 + lane immediate, push i32" shape as `ExtractLane`, just
    /// at `i8x16`'s narrower width and wider lane count -- the first
    /// `extract_lane` family member with a genuine signed/unsigned split
    /// (`i32x4.extract_lane` has none, since a full 32-bit lane has no
    /// narrower representation left to sign- or zero-extend from).
    ExtractLaneI8x16S,
    /// `i8x16.extract_lane_u` -- same shape as [`Self::ExtractLaneI8x16S`],
    /// but ZERO-extended to `i32` instead of sign-extended.
    ExtractLaneI8x16U,
    /// `i8x16.replace_lane` -- pop an `i32` (only its low byte is used)
    /// and a `v128`, overwrite the `v128`'s lane selected by a lane-index
    /// immediate (0-15) with that low byte, push the resulting `v128`. A
    /// GENUINELY NEW shape in this table, not a variant of any prior
    /// kind: every existing lane-immediate op ([`Self::ExtractLane`] and
    /// its `i8x16` siblings above) pops exactly ONE `v128` and produces a
    /// SCALAR; every existing mixed-type binary op (the `ShlI8x16`-family
    /// shifts) has no lane immediate at all. This is the first kind that
    /// combines BOTH a lane-index immediate AND a binary pop of two
    /// DIFFERENT operand types (`v128`, then `i32`), producing a `v128`
    /// result -- deliberately not force-fit into `ExtractLane`'s shape,
    /// since neither its pop count nor its result type match.
    ReplaceLaneI8x16,
    /// `i16x8.extract_lane_s` (SIMD widen PR37) -- read one `i16` lane
    /// back out of a `v128`, selected by a lane-index immediate (0-7,
    /// between `i8x16`'s 0-15 and `i32x4`'s 0-3), SIGN-extended to `i32`.
    /// Direct 8-lane mirror of [`Self::ExtractLaneI8x16S`], one lane
    /// width up -- same "pop v128 + lane immediate, push i32" shape,
    /// same signed/unsigned split (a 16-bit lane still has a narrower
    /// representation to sign- or zero-extend from, unlike `i32x4`'s
    /// full-width lanes).
    ExtractLaneI16x8S,
    /// `i16x8.extract_lane_u` -- same shape as [`Self::ExtractLaneI16x8S`],
    /// but ZERO-extended to `i32` instead of sign-extended.
    ExtractLaneI16x8U,
    /// `i16x8.replace_lane` -- pop an `i32` (only its low 16 bits are
    /// used) and a `v128`, overwrite the `v128`'s lane selected by a
    /// lane-index immediate (0-7) with that low `i16`, push the
    /// resulting `v128`. Direct 8-lane mirror of
    /// [`Self::ReplaceLaneI8x16`], one lane width up -- same mixed-type
    /// (`v128`, then `i32`) binary pop producing a `v128`.
    ReplaceLaneI16x8,
    /// `i32x4.replace_lane` (SIMD widen PR37) -- pop an `i32` and a
    /// `v128`, overwrite the `v128`'s lane selected by a lane-index
    /// immediate (0-3) with the full `i32`, push the resulting `v128`.
    /// The `i32x4` counterpart to [`Self::ExtractLane`] (which reads
    /// `i32x4` lanes back out) -- same shape as
    /// [`Self::ReplaceLaneI8x16`]/[`Self::ReplaceLaneI16x8`], just at
    /// `i32x4`'s full 32-bit lane width, so there's no narrowing
    /// truncation on write (unlike the `i8x16`/`i16x8` replace variants,
    /// which only keep the low byte/half-word of the popped `i32`).
    ReplaceLaneI32x4,
    /// `i64x2.extract_lane` (SIMD widen PR37) -- read one `i64` lane
    /// back out of a `v128`, selected by a lane-index immediate (0-1,
    /// this table's narrowest lane-index range so far). Pops a `v128`,
    /// pushes an `I64` -- the first `extract_lane` family member whose
    /// result is NOT `i32`: a full 64-bit lane is already the native
    /// WASM `i64` stack type, so there's no widening left to do (unlike
    /// `i8x16`/`i16x8`, which sign-/zero-extend their narrower lanes up
    /// to `i32`), and no signed/unsigned split either.
    ExtractLaneI64x2,
    /// `i64x2.replace_lane` -- pop an `i64` and a `v128`, overwrite the
    /// `v128`'s lane selected by a lane-index immediate (0-1) with that
    /// `i64`, push the resulting `v128`. Same mixed-type binary-pop
    /// shape as [`Self::ReplaceLaneI8x16`]/[`Self::ReplaceLaneI16x8`]/
    /// [`Self::ReplaceLaneI32x4`], but the FIRST `replace_lane` member
    /// whose popped scalar is `I64`, not `I32`.
    ReplaceLaneI64x2,
    /// `f32x4.extract_lane` (SIMD widen PR37) -- read one `f32` lane back
    /// out of a `v128`, selected by a lane-index immediate (0-3). Pops a
    /// `v128`, pushes an `F32` -- the first `extract_lane` family member
    /// whose result is FLOATING-POINT, not integer. No sign-/zero-
    /// extension split (floating-point lanes have no narrower-width
    /// variant to extend from), same reasoning as `i32x4`/`i64x2`'s
    /// single-variant extract ops.
    ExtractLaneF32x4,
    /// `f32x4.replace_lane` -- pop an `f32` and a `v128`, overwrite the
    /// `v128`'s lane selected by a lane-index immediate (0-3) with that
    /// `f32`, push the resulting `v128`. Same mixed-type binary-pop
    /// shape as the integer `replace_lane` variants, but the FIRST
    /// `replace_lane` member whose popped scalar is a FLOAT.
    ReplaceLaneF32x4,
    /// `f64x2.extract_lane` (SIMD widen PR37) -- read one `f64` lane back
    /// out of a `v128`, selected by a lane-index immediate (0-1, same
    /// narrow range as `i64x2`'s extract/replace pair, since both are
    /// this table's only 2-lane shapes). Pops a `v128`, pushes an `F64`.
    ExtractLaneF64x2,
    /// `f64x2.replace_lane` -- pop an `f64` and a `v128`, overwrite the
    /// `v128`'s lane selected by a lane-index immediate (0-1) with that
    /// `f64`, push the resulting `v128`. The LAST member of the
    /// `extract_lane`/`replace_lane` family across all six SIMD vector
    /// shapes (`i8x16`/`i16x8`/`i32x4`/`i64x2`/`f32x4`/`f64x2`) --
    /// closes out the family this table opened with `i32x4.extract_lane`
    /// back in SIMD PR1b-2.
    ReplaceLaneF64x2,
    /// `f32x4.abs` -- pop one `v128`, clear the sign bit of each of the 4
    /// `f32` lanes (`f32::abs()` in Rust is a pure bit operation here, no
    /// NaN/signed-zero subtlety -- unlike [`Self::MinF32x4`] below). Same
    /// UNARY "pop v128, push v128" shape as [`Self::AbsI8x16`], just at
    /// `f32x4`'s lane width -- the first FLOATING-POINT-typed unary
    /// arithmetic op in this table, following on from PR17's
    /// [`Self::SplatF32x4`]/[`Self::SplatF64x2`] (pure bit-pattern
    /// broadcasts, no arithmetic) and PR18's integer-only unary/binary
    /// arith widening.
    AbsF32x4,
    /// `f32x4.mul` -- pop two `v128`s, multiply each of the 4 `f32` lane
    /// pairs with standard IEEE-754 float multiply (Rust's `*` on `f32`
    /// is correct here -- ordinary multiplication has no WASM-specific
    /// NaN/signed-zero deviation from IEEE-754, unlike `min`/`max`). Same
    /// BINARY "pop two v128s, push one" shape as [`Self::MulI16x8`], just
    /// at `f32x4`'s lane width.
    MulF32x4,
    /// `f32x4.min` -- pop two `v128`s, take the WASM-spec `fmin` of each
    /// of the 4 `f32` lane pairs. Same BINARY shape as [`Self::MulF32x4`]
    /// above, but NOT a plain `f32::min()`/`a.min(b)`: WASM's `fmin` is
    /// NOT IEEE `minNum` -- if EITHER operand is NaN the result is NaN
    /// (propagated, not silently dropped the way Rust's native
    /// `f32::min()` drops one NaN operand and returns the other), and for
    /// a `-0.0`/`+0.0` tie, `-0.0` wins (unlike some other `minNum`
    /// variants that pick `+0.0` or are unspecified on the tie). This is
    /// the exact per-lane transplant of this crate's own scalar
    /// `f32.min`/`f64.min` opcode handlers (0x96/0xA4 in
    /// `wasm-execution`), which already implement this correct
    /// NaN-propagating, signed-zero-aware `fmin` for the non-SIMD MVP
    /// opcodes -- see that handler's own comment for the bug this
    /// discipline fixes (`min(NaN, -0.0)` silently returning `-0.0`
    /// instead of `NaN` under Rust's native `.min()`).
    MinF32x4,
    /// `f32x4.neg` -- pop one `v128`, flip the sign bit of each of the 4
    /// `f32` lanes (`-v` in Rust is a pure bit operation here, same "no
    /// NaN/signed-zero subtlety" discipline as [`Self::AbsF32x4`] --
    /// `-NaN` is still NaN, just with its sign bit flipped, which is the
    /// spec-correct result, not an error case to special-case). Same
    /// UNARY "pop v128, push v128" shape as [`Self::AbsF32x4`], just
    /// negating instead of clearing the sign bit.
    NegF32x4,
    /// `f32x4.sqrt` -- pop one `v128`, take the IEEE-754 square root of
    /// each of the 4 `f32` lanes (`f32::sqrt()` in Rust is directly
    /// correct here: it's already IEEE-754 compliant, including
    /// `sqrt(negative) == NaN` and `sqrt(-0.0) == -0.0`, so -- like
    /// [`Self::MulF32x4`] below, and UNLIKE [`Self::MinF32x4`] above --
    /// no bespoke NaN/signed-zero handling is needed). Same UNARY "pop
    /// v128, push v128" shape as [`Self::AbsF32x4`]/[`Self::NegF32x4`],
    /// the first genuinely non-bitwise unary `f32x4` arithmetic op in
    /// this table.
    SqrtF32x4,
    /// `f32x4.add` -- pop two `v128`s, add each of the 4 `f32` lane pairs
    /// with standard IEEE-754 float addition (Rust's `+` on `f32` is
    /// correct here -- ordinary addition has no WASM-specific deviation
    /// from IEEE-754, unlike `min`/`max`). Same BINARY "pop two v128s,
    /// push one" shape as [`Self::MulF32x4`], just addition instead of
    /// multiplication.
    AddF32x4,
    /// `f32x4.sub` -- same BINARY shape as [`Self::AddF32x4`], but
    /// standard IEEE-754 float subtraction (Rust's `-` on `f32`) of each
    /// of the 4 `f32` lane pairs instead of addition.
    SubF32x4,
    /// `f32x4.div` -- same BINARY shape as [`Self::AddF32x4`]/
    /// [`Self::SubF32x4`], but standard IEEE-754 float division (Rust's
    /// `/` on `f32`) of each of the 4 `f32` lane pairs. IEEE-754 division
    /// is TOTAL, not partial: a finite lane divided by `0.0` produces
    /// `+/-infinity` (sign per the usual sign-of-quotient rule, including
    /// signed zero divisors), and `0.0 / 0.0` produces `NaN` -- Rust's
    /// native `f32` division already implements this exactly, so there is
    /// NO trap and NO panic on a zero divisor (unlike this crate's
    /// integer division opcodes, which do trap on divide-by-zero -- float
    /// division is a fundamentally different, total operation).
    DivF32x4,
    /// `i32x4.trunc_sat_f32x4_s` -- pop one `v128`, convert each of the 4
    /// `f32` lanes to a SIGNED SATURATING `i32`, push one `v128`. This is
    /// the SIMD counterpart of the `0xFC`-prefixed scalar
    /// `i32.trunc_sat_f32_s` instruction (sub-opcode `0x00`, the
    /// "non-trapping float-to-int conversions" proposal) -- crucially,
    /// like that scalar op and UNLIKE this crate's TRAPPING
    /// `i32.trunc_f32_s`/`_u` MVP opcodes (`0xA8`/`0xA9`), `trunc_sat`
    /// NEVER TRAPS: a NaN lane saturates to `0`, a lane below
    /// `i32::MIN` saturates to `i32::MIN`, a lane above `i32::MAX`
    /// saturates to `i32::MAX`, and an ordinary in-range lane truncates
    /// toward zero same as the trapping version. Rust's own `as` cast
    /// from `f32` to `i32` has implemented exactly this saturating
    /// semantic since Rust 1.45 (the "T-as-int" RFC), so `lane as i32`
    /// is directly correct -- no hand-rolled NaN/range checks needed,
    /// mirroring the discipline the `0xFC` scalar `trunc_sat` handlers
    /// in `wasm-execution` already use. Same UNARY "pop v128, push
    /// v128" shape as [`Self::AbsF32x4`], just with a lane-width-
    /// preserving type change (f32 lane in, i32 lane out) instead of a
    /// same-type transform.
    TruncSatF32x4S,
    /// `i32x4.trunc_sat_f32x4_u` -- same UNARY shape as
    /// [`Self::TruncSatF32x4S`], but each `f32` lane saturates to an
    /// UNSIGNED `i32` interpretation instead: a NaN lane saturates to
    /// `0`, a lane below `0` saturates to `0` (NOT wrapped/reinterpreted
    /// -- a negative float genuinely means "below the unsigned range's
    /// minimum"), a lane above `u32::MAX` saturates to `u32::MAX`, and
    /// an ordinary in-range lane truncates toward zero. Rust's `as` cast
    /// from `f32` to `u32` has the same saturating semantics as the `_s`
    /// case, so `lane as u32` is directly correct; the result is then
    /// stored as a `u32` bit pattern into the same 4-byte lane slot
    /// this table's other `i32x4`-lane ops use (the lane storage itself
    /// doesn't distinguish signed/unsigned -- only the conversion that
    /// produced it does).
    TruncSatF32x4U,
    /// `f32x4.convert_i32x4_s` -- pop one `v128`, convert each of the 4
    /// `i32` lanes (interpreted as SIGNED) to `f32`, push one `v128`.
    /// The inverse direction of [`Self::TruncSatF32x4S`]. Rust's `as`
    /// cast from `i32` to `f32` already performs the correct
    /// round-to-nearest, ties-to-even conversion WASM's spec requires,
    /// so `lane as f32` (reading the lane's bytes as `i32` first) is
    /// directly correct -- the simpler of this PR's two `convert`
    /// directions, since no bit-pattern reinterpretation is needed
    /// before the cast (contrast [`Self::ConvertI32x4U`] below, which
    /// DOES need one). Same UNARY "pop v128, push v128" shape as
    /// [`Self::TruncSatF32x4S`], just the reverse type change (i32 lane
    /// in, f32 lane out).
    ConvertI32x4S,
    /// `f32x4.convert_i32x4_u` -- same UNARY shape as
    /// [`Self::ConvertI32x4S`], but each `i32` lane's bit pattern is
    /// reinterpreted as UNSIGNED (`u32`) BEFORE the conversion to
    /// `f32`, not converted directly from the signed `i32`. This
    /// matters: a lane with the high bit set (e.g. the bit pattern
    /// `0xFFFFFFFF`, which as a signed `i32` is `-1`) must convert to
    /// `4294967295.0f32` (`u32::MAX`), NOT `-1.0f32` -- so the runtime
    /// handler must do `(lane_bytes_as_i32 as u32) as f32`, in that
    /// order, never `lane_bytes_as_i32 as f32` directly (which would
    /// sign-extend the bit pattern into the wrong float value for any
    /// lane with the high bit set).
    ConvertI32x4U,
    /// `i64x2.extmul_low_i32x4_s` -- reinterpret both `v128` operands as 4
    /// SIGNED `i32` lanes each; take only the LOW 2 lanes (indices 0-1) of
    /// each, sign-extend every value to `i64`, and multiply the
    /// corresponding pairs lane-wise, producing an `i64x2` result. The
    /// third and final rung of this table's widening-multiply "extmul"
    /// family, mirroring [`Self::ExtmulLowI16x8S`] (which itself mirrors
    /// [`Self::ExtmulLowI8x16S`]) one lane width up: same narrow-input/
    /// wide-output BINARY shape, just `i32x4` -> `i64x2` instead of
    /// `i16x8` -> `i32x4`.
    ExtmulLowI64x2S,
    /// `i64x2.extmul_high_i32x4_s` -- same as [`Self::ExtmulLowI64x2S`],
    /// but operates on the HIGH 2 lanes (indices 2-3) of each `i32x4`
    /// operand instead of the low 2.
    ExtmulHighI64x2S,
    /// `i64x2.extmul_low_i32x4_u` -- same LOW-2-lanes widening multiply as
    /// [`Self::ExtmulLowI64x2S`], but each `i32` lane is zero-extended
    /// (read as `u32`) before the multiply, not sign-extended.
    ExtmulLowI64x2U,
    /// `i64x2.extmul_high_i32x4_u` -- same HIGH-2-lanes widening multiply
    /// as [`Self::ExtmulHighI64x2S`], but zero-extended, not
    /// sign-extended.
    ExtmulHighI64x2U,
    /// `i16x8.q15mulr_sat_s` -- pop two `v128`s, treat each of the 8 `i16`
    /// lane pairs as SIGNED Q15 fixed-point values (the range
    /// `[-32768, 32767]` represents `[-1.0, ~1.0)` in Q15), and compute a
    /// ROUNDING SATURATING fixed-point multiply per lane, push one `v128`.
    /// A GENUINELY NEW shape/semantic in this table -- every prior BINARY
    /// "pop two v128s, push one" op ([`Self::MulI16x8`] and friends) is
    /// either a plain wrapping arithmetic op or a min/max/compare; this is
    /// the first FIXED-POINT rounding multiply. Per lane: sign-extend both
    /// `i16`s to `i32` (`a as i32 * b as i32` cannot overflow `i32` --
    /// max magnitude is `32768 * 32768 == 2^30`, well inside `i32::MAX ==
    /// 2^31 - 1`), add the Q15 rounding constant `0x4000` (`2^14`, i.e.
    /// "round to nearest" when rescaling from the Q30 product back to
    /// Q15), then arithmetic-shift right by 15. That rescaled value can
    /// exceed `i16::MAX` in exactly ONE case -- both lanes at
    /// `i16::MIN` (`-32768`): `(-32768 * -32768 + 0x4000) >> 15 ==
    /// 32768`, one past `i16::MAX` -- so the final step SATURATES
    /// (clamps) the result to `i16::MIN..=i16::MAX` rather than truncating
    /// or wrapping. This is the WASM "relaxed"-adjacent but actually
    /// MVP-SIMD `q15mulr_sat_s` instruction, popular in DSP/audio code for
    /// exactly this rounding fixed-point multiply.
    Q15mulrSatI16x8S,
    /// `i32x4.trunc_sat_f64x2_s_zero` -- pop one `v128`, read it as 2 `f64`
    /// lanes (not 4 `f32` lanes -- `f64x2` is a NARROWER-lane-COUNT, WIDER-
    /// lane-WIDTH shape than [`Self::TruncSatF32x4S`]'s `f32x4` operand),
    /// convert each to a SIGNED SATURATING `i32` the same way
    /// [`Self::TruncSatF32x4S`] does (NaN saturates to `0`, out-of-range
    /// saturates to `i32::MIN`/`i32::MAX`, in-range truncates toward zero,
    /// via Rust's saturating `as` cast -- `lane as i32`, no hand-rolled
    /// bounds checking needed, same Rust-1.45-since discipline as
    /// [`Self::TruncSatF32x4S`]'s own doc comment), and push one `v128`
    /// with 4 `i32` lanes: lanes 0-1 hold the two truncated results (in
    /// the SAME order as the source `f64` lanes), lanes 2-3 are always
    /// `0`. The "_zero" in this op's name refers to exactly that
    /// zero-fill of the upper half -- `f64x2` only has 2 lanes to widen
    /// `i32x4`'s 4, so the SIMD proposal defines this family as
    /// "truncate what you have, zero-extend the rest" rather than, say,
    /// repeating or NaN-filling the upper 2 lanes.
    TruncSatF64x2SZero,
    /// `i32x4.trunc_sat_f64x2_u_zero` -- same shape as
    /// [`Self::TruncSatF64x2SZero`] (2 `f64` lanes in, 4 `i32` lanes out,
    /// upper 2 always zero), but each `f64` lane saturates to an UNSIGNED
    /// `i32` interpretation instead, same signed/unsigned split as
    /// [`Self::TruncSatF32x4U`] vs. [`Self::TruncSatF32x4S`]: a NaN lane
    /// saturates to `0`, a lane below `0` saturates to `0` (not wrapped),
    /// a lane above `u32::MAX` saturates to `u32::MAX`, and the result is
    /// stored as a `u32` bit pattern in the same 4-byte lane slot every
    /// other `i32x4`-lane op in this table uses.
    TruncSatF64x2UZero,
    /// `i16x8.extend_low_i8x16_s` -- reinterpret the ONE popped `v128` as
    /// 16 `i8` lanes, take only the LOW 8 lanes (indices 0-7),
    /// sign-extend each to `i16`, producing an `i16x8` result. This is
    /// EXACTLY the lane-selection + sign-extend half of
    /// [`Self::ExtmulLowI8x16S`], minus the multiply -- UNARY where
    /// `extmul` is BINARY, same narrow-input (8-bit)/wide-output (16-bit)
    /// shape as [`Self::ExtaddPairwiseI8x16S`]. Part of the 16-opcode set
    /// (`extend_low`/`high`, `narrow`, `promote`/`demote`/`convert_low`)
    /// needed to unlock the upstream `simd_conversions.wast` corpus file
    /// -- see this crate's `SIMD_OPS` doc comment.
    ExtendLowI8x16S,
    /// `i16x8.extend_high_i8x16_s` -- same as [`Self::ExtendLowI8x16S`],
    /// but takes the HIGH 8 lanes (indices 8-15) of the operand `v128`
    /// instead of the low 8. Mirrors [`Self::ExtmulHighI8x16S`]'s lane
    /// selection, minus the multiply.
    ExtendHighI8x16S,
    /// `i16x8.extend_low_i8x16_u` -- same LOW-8-lanes shape as
    /// [`Self::ExtendLowI8x16S`], but each `i8` lane is zero-extended
    /// (read as `u8`) instead of sign-extended.
    ExtendLowI8x16U,
    /// `i16x8.extend_high_i8x16_u` -- same HIGH-8-lanes shape as
    /// [`Self::ExtendHighI8x16S`], but zero-extended, not sign-extended.
    ExtendHighI8x16U,
    /// `i32x4.extend_low_i16x8_s` -- reinterpret the ONE popped `v128` as
    /// 8 `i16` lanes, take only the LOW 4 lanes (indices 0-3),
    /// sign-extend each to `i32`, producing an `i32x4` result. Same
    /// pattern one lane width up from [`Self::ExtendLowI8x16S`] --
    /// EXACTLY the lane-selection + sign-extend half of
    /// [`Self::ExtmulLowI16x8S`], minus the multiply.
    ExtendLowI16x8S,
    /// `i32x4.extend_high_i16x8_s` -- same as [`Self::ExtendLowI16x8S`],
    /// but takes the HIGH 4 lanes (indices 4-7) of the operand `v128`
    /// instead of the low 4. Mirrors [`Self::ExtmulHighI16x8S`]'s lane
    /// selection, minus the multiply.
    ExtendHighI16x8S,
    /// `i32x4.extend_low_i16x8_u` -- same LOW-4-lanes shape as
    /// [`Self::ExtendLowI16x8S`], but each `i16` lane is zero-extended
    /// (read as `u16`) instead of sign-extended.
    ExtendLowI16x8U,
    /// `i32x4.extend_high_i16x8_u` -- same HIGH-4-lanes shape as
    /// [`Self::ExtendHighI16x8S`], but zero-extended, not sign-extended.
    ExtendHighI16x8U,
    /// `i64x2.extend_low_i32x4_s` -- reinterpret the ONE popped `v128` as
    /// 4 `i32` lanes, take only the LOW 2 lanes (indices 0-1),
    /// sign-extend each to `i64`, producing an `i64x2` result. Same
    /// pattern one lane width up from [`Self::ExtendLowI16x8S`] --
    /// EXACTLY the lane-selection + sign-extend half of
    /// [`Self::ExtmulLowI64x2S`], minus the multiply. Third and FINAL
    /// rung of the "extend" family (`i16x8`-from-`i8x16` in
    /// [`Self::ExtendLowI8x16S`], `i32x4`-from-`i16x8` in this same
    /// variant's `I16x8` sibling, `i64x2`-from-`i32x4` here).
    ExtendLowI32x4S,
    /// `i64x2.extend_high_i32x4_s` -- same as [`Self::ExtendLowI32x4S`],
    /// but takes the HIGH 2 lanes (indices 2-3) of the operand `v128`
    /// instead of the low 2. Mirrors [`Self::ExtmulHighI64x2S`]'s lane
    /// selection, minus the multiply.
    ExtendHighI32x4S,
    /// `i64x2.extend_low_i32x4_u` -- same LOW-2-lanes shape as
    /// [`Self::ExtendLowI32x4S`], but each `i32` lane is zero-extended
    /// (read as `u32`) instead of sign-extended.
    ExtendLowI32x4U,
    /// `i64x2.extend_high_i32x4_u` -- same HIGH-2-lanes shape as
    /// [`Self::ExtendHighI32x4S`], but zero-extended, not sign-extended.
    ExtendHighI32x4U,
    /// `i8x16.narrow_i16x8_s` -- BINARY (the opposite shape from
    /// [`Self::ExtendLowI8x16S`]'s UNARY): pop TWO `v128`s, each read as
    /// 8 `i16` lanes. Each `i16` lane of BOTH operands is SIGNED-
    /// saturated to the `i8` range (`i8::MIN..=i8::MAX`) and cast down
    /// to `i8`, producing one `i8x16` result -- the FIRST (bottom of
    /// stack) operand's 8 saturated lanes become the LOW half (indices
    /// 0-7) of the result, the SECOND (top of stack) operand's 8
    /// saturated lanes become the HIGH half (indices 8-15). This
    /// operand-to-half ordering is the classic bug spot for this
    /// opcode family -- verified against the WASM SIMD spec's own
    /// `narrow` pseudocode (`result[i] = Saturate(a[i])` for `i` in
    /// `0..N`, `result[N+i] = Saturate(b[i])` for `i` in `0..N`) before
    /// implementing. Part of the 16-opcode set (`extend_low`/`high`,
    /// `narrow`, `promote`/`demote`/`convert_low`) needed to unlock the
    /// upstream `simd_conversions.wast` corpus file -- see this crate's
    /// `SIMD_OPS` doc comment.
    NarrowI16x8S,
    /// `i8x16.narrow_i16x8_u` -- same BINARY/lane-ordering shape as
    /// [`Self::NarrowI16x8S`], but UNSIGNED-saturates each `i16` lane
    /// to `0..=u8::MAX` (255) instead of the signed `i8` range. The
    /// classic gotcha here: a negative `i16` lane (e.g. `-1`) saturates
    /// to `0`, NOT to a large unsigned value via bit-reinterpretation
    /// -- this is a genuine clamp on the lane's SIGNED value, not a
    /// wrapping cast.
    NarrowI16x8U,
    /// `i16x8.narrow_i32x4_s` -- same pattern one lane width up from
    /// [`Self::NarrowI16x8S`]: pop TWO `v128`s, each read as 4 `i32`
    /// lanes, SIGNED-saturate each to the `i16` range
    /// (`i16::MIN..=i16::MAX`), cast down to `i16`. First operand's 4
    /// saturated lanes -> LOW half (indices 0-3) of the `i16x8` result,
    /// second operand's 4 saturated lanes -> HIGH half (indices 4-7).
    NarrowI32x4S,
    /// `i16x8.narrow_i32x4_u` -- same shape as [`Self::NarrowI32x4S`],
    /// but UNSIGNED-saturates each `i32` lane to `0..=u16::MAX` (65535)
    /// instead of the signed `i16` range -- same "negative saturates to
    /// zero, not wraps" discipline as [`Self::NarrowI16x8U`].
    NarrowI32x4U,
    /// `f32x4.demote_f64x2_zero` -- pop one `v128`, read it as 2 `f64`
    /// lanes, demote (narrow) each to `f32` via the same plain `as f32`
    /// cast the scalar `f32.demote_f64` handler (0xB6) uses -- IEEE-754
    /// narrowing that CAN lose precision or overflow to `f32::INFINITY`/
    /// `f32::NEG_INFINITY` for out-of-range magnitudes (expected
    /// behavior, not an error), inheriting that scalar handler's NaN
    /// convention: no hand-rolled payload preservation, since Rust's `as`
    /// cast (LLVM's `fptrunc`) canonicalizes the NaN payload, exactly as
    /// this crate's `to_typed`/`from_typed` doc comment already documents
    /// for the SAME f64->f32 narrowing direction elsewhere in this
    /// codebase. Push one `v128` with 4 `f32` lanes: lanes 0-1 hold the
    /// two demoted results (SAME order as the source `f64` lanes), lanes
    /// 2-3 are always `0.0`. Mirrors [`Self::TruncSatF64x2SZero`]'s exact
    /// zero-fill shape -- "_zero" means the same thing here: `f64x2` only
    /// has 2 lanes to widen `f32x4`'s 4. Last of the 16-opcode
    /// `extend_low`/`high` (PR26) + `narrow` (PR27) + `promote`/`demote`/
    /// `convert_low` (this PR) set needed to unlock the upstream
    /// `simd_conversions.wast` corpus file -- see this crate's `SIMD_OPS`
    /// doc comment.
    DemoteF64x2Zero,
    /// `f64x2.promote_low_f32x4` -- pop one `v128`, read it as 4 `f32`
    /// lanes, take only the LOW 2 lanes (indices 0-1) -- lanes 2-3 are
    /// DROPPED, never read into the result (the opposite discipline from
    /// [`Self::DemoteF64x2Zero`]'s zero-FILL: this is lane-DROPPING,
    /// since promoting from 4 lanes to 2 can't invent extra output lanes
    /// to zero) -- and promote (widen) each to `f64` via the same plain
    /// `as f64` cast the scalar `f64.promote_f32` handler (0xBB) uses
    /// (exact, lossless IEEE-754 widening for every finite value; same
    /// NaN-payload-canonicalization caveat as [`Self::DemoteF64x2Zero`]
    /// documents). Push one `v128` with 2 `f64` lanes holding the two
    /// promoted results, in the same order as the source low-half `f32`
    /// lanes.
    PromoteLowF32x4,
    /// `f64x2.convert_low_i32x4_s` -- pop one `v128`, read it as 4 `i32`
    /// lanes, take only the LOW 2 lanes (indices 0-1, same lane-DROPPING
    /// discipline as [`Self::PromoteLowF32x4`] -- lanes 2-3 are never
    /// read), convert each SIGNED `i32` to `f64` via `v as f64` (exact
    /// and lossless -- every `i32` value fits precisely in `f64`'s
    /// 52-bit mantissa, so there's no rounding or NaN case to consider
    /// at all, unlike the float-source conversions above). Push one
    /// `v128` with 2 `f64` lanes holding the two converted results, in
    /// the same order as the source low-half `i32` lanes. The reverse
    /// direction of [`Self::TruncSatF64x2SZero`] (that went `f64x2` ->
    /// `i32x4` with zero-PADDING; this goes `i32x4` -> `f64x2` with
    /// lane-DROPPING).
    ConvertLowI32x4S,
    /// `f64x2.convert_low_i32x4_u` -- same LOW-2-lanes-dropping shape as
    /// [`Self::ConvertLowI32x4S`], but each `i32` lane's bit pattern is
    /// reinterpreted as `u32` BEFORE the `f64` conversion (`(v as u32)
    /// as f64`), same signed/unsigned split as
    /// [`Self::ConvertI32x4U`]/[`Self::ConvertI32x4S`] -- a lane with the
    /// high bit set (e.g. bit pattern `0xFFFFFFFF`, i.e. `-1` as a
    /// signed `i32`) must convert to `4294967295.0f64` (`u32::MAX`), NOT
    /// `-1.0f64`.
    ConvertLowI32x4U,
    /// `f32x4.eq` -- lane-wise IEEE-754 equality of 4 `f32` lane pairs,
    /// boolean mask (all-1s/all-0s per lane). BINARY, same
    /// pop-two-push-one shape and mask convention as [`Self::Eq`]
    /// (`i32x4.eq`), but the FIRST floating-point comparison family in
    /// this table -- unlike the integer families, there is no signed/
    /// unsigned split (IEEE-754 comparison has only one notion of
    /// "equal"), and a NaN operand on EITHER side makes every comparison
    /// in this family false EXCEPT `ne` (see [`Self::NeF32x4`]). Native
    /// Rust `f32` `==` already implements this correctly (including
    /// `+0.0 == -0.0` being true and `NaN == NaN` being false), so the
    /// handler needs no bespoke NaN-detection logic, just the operator
    /// itself.
    EqF32x4,
    /// `f32x4.ne` -- lane-wise IEEE-754 inequality, boolean mask. BINARY,
    /// same shape as [`Self::EqF32x4`]. The one family member where a NaN
    /// operand makes the result TRUE (not false): IEEE-754 "unordered"
    /// comparisons report `!=` as true whenever either operand is NaN,
    /// including a lane compared with itself. Native Rust `f32` `!=`
    /// already implements this correctly.
    NeF32x4,
    /// `f32x4.lt` -- lane-wise IEEE-754 ordered less-than, boolean mask.
    /// BINARY, same shape as [`Self::EqF32x4`]. Any NaN operand makes the
    /// result false (an "unordered" comparison is never less-than).
    /// Native Rust `f32` `<` already implements this correctly.
    LtF32x4,
    /// `f32x4.gt` -- lane-wise IEEE-754 ordered greater-than, boolean
    /// mask. BINARY, same shape as [`Self::EqF32x4`], same NaN-is-false
    /// discipline as [`Self::LtF32x4`]. Native Rust `f32` `>` already
    /// implements this correctly.
    GtF32x4,
    /// `f32x4.le` -- lane-wise IEEE-754 ordered less-than-or-equal,
    /// boolean mask. BINARY, same shape as [`Self::EqF32x4`], same
    /// NaN-is-false discipline as [`Self::LtF32x4`]. Native Rust `f32`
    /// `<=` already implements this correctly.
    LeF32x4,
    /// `f32x4.ge` -- lane-wise IEEE-754 ordered greater-than-or-equal,
    /// boolean mask. BINARY, same shape as [`Self::EqF32x4`], same
    /// NaN-is-false discipline as [`Self::LtF32x4`]. Native Rust `f32`
    /// `>=` already implements this correctly.
    GeF32x4,
    /// `f64x2.neg` -- pop one `v128`, flip the sign bit of each of the 2
    /// `f64` lanes (`-v` in Rust is a pure bit operation here, same "no
    /// NaN/signed-zero subtlety" discipline as [`Self::NegF32x4`] --
    /// `-NaN` is still NaN, just with its sign bit flipped, the
    /// spec-correct result). Same UNARY "pop v128, push v128" shape as
    /// [`Self::NegF32x4`], just at `f64x2`'s lane width (2 lanes of 8
    /// bytes each instead of 4 lanes of 4 bytes).
    NegF64x2,
    /// `f64x2.sqrt` -- pop one `v128`, take the IEEE-754 square root of
    /// each of the 2 `f64` lanes (`f64::sqrt()` in Rust is directly
    /// correct here: already IEEE-754 compliant, including
    /// `sqrt(negative) == NaN` and `sqrt(-0.0) == -0.0`, so -- like
    /// [`Self::SqrtF32x4`] -- no bespoke NaN/signed-zero handling is
    /// needed). Same UNARY shape as [`Self::NegF64x2`].
    SqrtF64x2,
    /// `f64x2.add` -- pop two `v128`s, add each of the 2 `f64` lane pairs
    /// with standard IEEE-754 float addition (Rust's `+` on `f64` is
    /// correct here -- ordinary addition has no WASM-specific deviation
    /// from IEEE-754, unlike `min`/`max`). Same BINARY "pop two v128s,
    /// push one" shape as [`Self::AddF32x4`], just at `f64x2`'s lane
    /// width.
    AddF64x2,
    /// `f64x2.sub` -- same BINARY shape as [`Self::AddF64x2`], but
    /// standard IEEE-754 float subtraction (Rust's `-` on `f64`) of each
    /// of the 2 `f64` lane pairs instead of addition.
    SubF64x2,
    /// `f64x2.mul` -- same BINARY shape as [`Self::AddF64x2`], but
    /// standard IEEE-754 float multiplication (Rust's `*` on `f64`) of
    /// each of the 2 `f64` lane pairs. Unlike [`Self::AddF64x2`]/
    /// [`Self::SubF64x2`]/[`Self::DivF64x2`], this is the first `f64x2`
    /// PR to introduce `mul` -- `f32x4.mul` already existed before the
    /// `f32x4` arithmetic-family PR, but no `f64x2.mul` existed until now,
    /// so it rides along here using the exact same binary-op boilerplate.
    MulF64x2,
    /// `f64x2.div` -- same BINARY shape as [`Self::AddF64x2`]/
    /// [`Self::SubF64x2`]/[`Self::MulF64x2`], but standard IEEE-754 float
    /// division (Rust's `/` on `f64`) of each of the 2 `f64` lane pairs.
    /// IEEE-754 division is TOTAL, not partial: a finite lane divided by
    /// `0.0` produces `+/-infinity` (sign per the usual sign-of-quotient
    /// rule, including signed zero divisors), and `0.0 / 0.0` produces
    /// `NaN` -- Rust's native `f64` division already implements this
    /// exactly, so there is NO trap and NO panic on a zero divisor, same
    /// discipline as [`Self::DivF32x4`].
    DivF64x2,
    /// `f64x2.eq` -- pop two `v128`s, compare each of the 2 `f64` lane
    /// pairs with ordinary IEEE-754 equality, push one `v128` boolean
    /// mask (all-1s per lane if equal, all-0s otherwise). Direct 2-lane
    /// mirror of [`Self::EqF32x4`] -- same BINARY shape, same mask
    /// convention, same "native operator is already correct" discipline
    /// (Rust's `f64` `==` already treats `+0.0 == -0.0` as true and any
    /// `NaN` comparison as false). This family's NaN-is-false rule holds
    /// for every member EXCEPT `ne` (see [`Self::NeF64x2`]). Native
    /// Rust `f64` `==` already implements this correctly.
    EqF64x2,
    /// `f64x2.ne` -- pop two `v128`s, compare each of the 2 `f64` lane
    /// pairs with ordinary IEEE-754 inequality, push one `v128` boolean
    /// mask. Direct 2-lane mirror of [`Self::NeF32x4`] -- BINARY,
    /// same shape as [`Self::EqF64x2`]. The one family member where a NaN
    /// operand makes the result TRUE (including a lane compared with
    /// itself), per IEEE-754's unordered-comparison rule. Native Rust
    /// `f64` `!=` already implements this correctly.
    NeF64x2,
    /// `f64x2.lt` -- pop two `v128`s, compare each of the 2 `f64` lane
    /// pairs with ordinary IEEE-754 ordered less-than, push one `v128`
    /// boolean mask. Direct 2-lane mirror of [`Self::LtF32x4`].
    /// BINARY, same shape as [`Self::EqF64x2`]. Any NaN operand makes the
    /// result false. Native Rust `f64` `<` already implements this
    /// correctly.
    LtF64x2,
    /// `f64x2.gt` -- pop two `v128`s, compare each of the 2 `f64` lane
    /// pairs with ordinary IEEE-754 ordered greater-than, push one `v128`
    /// boolean mask. Direct 2-lane mirror of [`Self::GtF32x4`].
    /// BINARY, same shape as [`Self::EqF64x2`], same NaN-is-false
    /// discipline as [`Self::LtF64x2`]. Native Rust `f64` `>` already
    /// implements this correctly.
    GtF64x2,
    /// `f64x2.le` -- pop two `v128`s, compare each of the 2 `f64` lane
    /// pairs with ordinary IEEE-754 ordered less-than-or-equal, push one
    /// `v128` boolean mask. Direct 2-lane mirror of [`Self::LeF32x4`].
    /// BINARY, same shape as [`Self::EqF64x2`], same
    /// NaN-is-false discipline as [`Self::LtF64x2`]. Native Rust `f64`
    /// `<=` already implements this correctly.
    LeF64x2,
    /// `f64x2.ge` -- pop two `v128`s, compare each of the 2 `f64` lane
    /// pairs with ordinary IEEE-754 ordered greater-than-or-equal, push
    /// one `v128` boolean mask. Direct 2-lane mirror of
    /// [`Self::GeF32x4`]. BINARY, same shape as [`Self::EqF64x2`], same
    /// NaN-is-false discipline as [`Self::LtF64x2`]. Native Rust `f64`
    /// `>=` already implements this correctly.
    GeF64x2,
    /// `i8x16.add_sat_s` -- pop two `v128`s, each read as 16 `i8` lanes.
    /// Same BINARY pop-order/lane-count shape as [`Self::AddI8x16`], but
    /// the sum of each lane pair is computed in a WIDER intermediate type
    /// (`i16`, never overflows -- `i8` magnitudes are at most 127, so
    /// `127 + 127 == 254` sits nowhere near `i16`'s `+/-32767` bound)
    /// then SIGNED-saturated to `i8::MIN..=i8::MAX` before the narrowing
    /// cast back to `i8` -- unlike `AddI8x16`'s `wrapping_add`, an
    /// out-of-range sum here clamps instead of wrapping. Same
    /// compute-in-a-wider-type-then-clamp discipline as
    /// [`Self::NarrowI16x8S`], except the wider-typed value is produced
    /// by an add here rather than being the operand itself.
    AddSatI8x16S,
    /// `i8x16.add_sat_u` -- same BINARY/wider-intermediate shape as
    /// [`Self::AddSatI8x16S`], but both lanes are read as UNSIGNED `u8`
    /// (zero-extended into `u16`, sum computed in `u16` -- never
    /// overflows, `255 + 255 == 510` is well inside `u16`'s `65535`
    /// bound), then UNSIGNED-saturated to `0..=u8::MAX` (255) before the
    /// narrowing cast back to `u8`. Same "compute unsigned in a wider
    /// unsigned type" discipline as [`Self::NarrowI16x8U`], adapted to a
    /// binary add instead of a single wide operand.
    AddSatI8x16U,
    /// `i8x16.sub_sat_s` -- same BINARY/wider-intermediate shape as
    /// [`Self::AddSatI8x16S`], but subtracts (`l as i16 - r as i16`)
    /// instead of adding -- the difference is likewise always
    /// representable in `i16` (max magnitude `127 - (-128) == 255`),
    /// then SIGNED-saturated to `i8::MIN..=i8::MAX`.
    SubSatI8x16S,
    /// `i8x16.sub_sat_u` -- same BINARY/wider-intermediate shape as
    /// [`Self::AddSatI8x16U`], but subtracts (`l as u16 - r as u16`,
    /// computed via `i32` to avoid an unsigned-subtraction underflow
    /// panic when `r > l`) instead of adding, then UNSIGNED-saturated to
    /// `0..=u8::MAX`. This is the classic bug spot for the whole
    /// saturating family: an underflowing unsigned subtraction (e.g.
    /// `3u8 - 10u8`) must clamp to `0`, NOT wrap around to a large
    /// unsigned byte (e.g. `249`) the way [`Self::SubI8x16`]'s
    /// `wrapping_sub` deliberately does -- explicit unit tests cover
    /// this direction.
    SubSatI8x16U,
    /// `i16x8.add_sat_s` -- same SIGNED-saturating-add shape as
    /// [`Self::AddSatI8x16S`], one lane width up: pop two `v128`s, each
    /// read as 8 `i16` lanes, sum each pair in a wider `i32` intermediate
    /// (never overflows -- `i16` magnitudes are at most 32767, so
    /// `32767 + 32767 == 65534` sits nowhere near `i32`'s `+/-2^31` bound),
    /// SIGNED-saturate to `i16::MIN..=i16::MAX`, cast down to `i16`.
    /// Direct mirror of [`Self::AddSatI8x16S`] at `i16x8` width.
    AddSatI16x8S,
    /// `i16x8.add_sat_u` -- same UNSIGNED-saturating-add shape as
    /// [`Self::AddSatI8x16U`], one lane width up: both lanes read as `u16`
    /// (zero-extended into `u32`, sum computed in `u32` -- never
    /// overflows, `65535 + 65535 == 131070` is well inside `u32`'s bound),
    /// UNSIGNED-saturated to `0..=u16::MAX` (65535). Direct mirror of
    /// [`Self::AddSatI8x16U`] at `i16x8` width.
    AddSatI16x8U,
    /// `i16x8.sub_sat_s` -- same SIGNED-saturating-subtract shape as
    /// [`Self::SubSatI8x16S`], one lane width up: subtracts in `i32`
    /// (max magnitude `32767 - (-32768) == 65535`, always representable),
    /// SIGNED-saturates to `i16::MIN..=i16::MAX`.
    SubSatI16x8S,
    /// `i16x8.sub_sat_u` -- same UNSIGNED-saturating-subtract shape as
    /// [`Self::SubSatI8x16U`], one lane width up: subtracts in `i64` (to
    /// avoid an unsigned-subtraction underflow panic when `r > l`, same
    /// discipline as [`Self::SubSatI8x16U`]'s `i32` intermediate),
    /// UNSIGNED-saturates to `0..=u16::MAX`. Same "underflow clamps to
    /// zero, does NOT wrap" discipline as every other `_u` member of this
    /// family -- explicit unit tests cover this direction at this lane
    /// width too.
    SubSatI16x8U,
    /// `f32x4.max` (SIMD widen PR34) -- pop two `v128`s, take the
    /// WASM-spec `fmax` of each of the 4 `f32` lane pairs. Same BINARY
    /// shape as [`Self::MinF32x4`] above, and the same "NOT a plain
    /// `f32::max()`" caveat applies in the mirror-image direction: WASM's
    /// `fmax` is NOT IEEE `maxNum` -- if EITHER operand is NaN the result
    /// is NaN (propagated), and for a `-0.0`/`+0.0` tie, `+0.0` wins
    /// (the opposite tie-break from [`Self::MinF32x4`]'s `-0.0`). This is
    /// the exact per-lane transplant of this crate's own scalar `f32.max`
    /// opcode handler (0x97 in `wasm-execution`), which already
    /// implements this correct NaN-propagating, signed-zero-aware `fmax`
    /// for the non-SIMD MVP opcode.
    MaxF32x4,
    /// `f32x4.pmin` (SIMD widen PR34) -- pop two `v128`s (`a` then `b`,
    /// in that push order so `a` is BELOW `b` on the stack, i.e. `a` was
    /// pushed first), compute each of the 4 lanes as `b < a ? b : a`
    /// using the IEEE-754 `<` operator DIRECTLY, push one `v128`. This is
    /// a "pseudo-min", DELIBERATELY SIMPLER than [`Self::MinF32x4`]
    /// above and NOT the same code path: no NaN canonicalization, no
    /// signed-zero tie-break special case -- just a plain conditional
    /// select. Since IEEE-754 `<` is always `false` when either operand
    /// is NaN, `pmin` returns `a` (the FIRST operand) unchanged whenever
    /// either operand is NaN, NOT a canonicalized NaN result the way
    /// `MinF32x4` would produce -- this first-operand-wins-on-NaN
    /// behavior is `pmin`'s whole reason for existing (a cheap select
    /// hardware can implement as one branchless compare-and-select
    /// instruction, unlike `min`'s NaN/signed-zero special-casing) and is
    /// the classic point of confusion/bugs porting real WASM SIMD
    /// implementations: copying `MinF32x4`'s NaN-canonicalization logic
    /// here would be WRONG.
    PminF32x4,
    /// `f32x4.pmax` (SIMD widen PR34) -- same "pseudo-max" shape as
    /// [`Self::PminF32x4`] above, symmetric formula: pop two `v128`s
    /// (`a` then `b`), compute each of the 4 lanes as `a < b ? b : a`
    /// using IEEE-754 `<` DIRECTLY, push one `v128`. Since `a < b` is
    /// always `false` when either operand is NaN, `pmax` also returns
    /// `a` (the FIRST operand) unchanged whenever either operand is NaN
    /// -- same first-operand-wins-on-NaN discipline as `pmin`, NOT
    /// [`Self::MaxF32x4`]'s NaN-canonicalizing behavior. Deliberately NOT
    /// implemented by reusing `MaxF32x4`'s NaN logic.
    PmaxF32x4,
    /// `f64x2.abs` (SIMD widen PR35) -- pop one `v128`, clear the sign bit
    /// of each of the 2 `f64` lanes, push one `v128`. A pure bit operation
    /// with no NaN/signed-zero subtlety -- `f64::abs()` is correct here,
    /// exactly like [`Self::AbsF32x4`]. Same UNARY "pop v128, push v128"
    /// shape as [`Self::NegF64x2`]/[`Self::SqrtF64x2`], just clearing the
    /// sign bit instead of flipping it or taking a square root. `f64x2`
    /// never got an `abs` handler in the earlier `f64x2` arithmetic-family
    /// PR (PR31 added `neg`/`sqrt`/`add`/`sub`/`mul`/`div` but not `abs`),
    /// so this PR fills that gap alongside `min`/`max`/`pmin`/`pmax`.
    AbsF64x2,
    /// `f64x2.min` (SIMD widen PR35) -- pop two `v128`s, take the
    /// WASM-spec `fmin` (NOT Rust's `f64::min()`/IEEE `minNum`) of each of
    /// the 2 `f64` lane pairs. Direct 2-lane mirror of [`Self::MinF32x4`]
    /// -- same BINARY shape, same NaN-propagating (either operand NaN ->
    /// result NaN), signed-zero-aware (-0.0 wins a -0.0/+0.0 tie) `fmin`
    /// discipline, just at `f64x2`'s lane width. This is the exact
    /// per-lane transplant of this crate's own scalar `f64.min` opcode
    /// handler, same "NOT a plain native `.min()`" caveat as `MinF32x4`.
    MinF64x2,
    /// `f64x2.max` (SIMD widen PR35) -- pop two `v128`s, take the
    /// WASM-spec `fmax` (NOT Rust's `f64::max()`/IEEE `maxNum`) of each of
    /// the 2 `f64` lane pairs. Direct 2-lane mirror of [`Self::MaxF32x4`]
    /// -- same BINARY shape, same NaN-propagating, signed-zero-aware
    /// (+0.0 wins a -0.0/+0.0 tie, the mirror-image tie-break from
    /// [`Self::MinF64x2`]'s -0.0) `fmax` discipline, just at `f64x2`'s
    /// lane width.
    MaxF64x2,
    /// `f64x2.pmin` (SIMD widen PR35) -- pop two `v128`s (`a` then `b`, in
    /// that push order so `a` was pushed first), compute each of the 2
    /// lanes as `b < a ? b : a` using the IEEE-754 `<` operator DIRECTLY,
    /// push one `v128`. Direct 2-lane mirror of [`Self::PminF32x4`] --
    /// DELIBERATELY SIMPLER than [`Self::MinF64x2`] above and NOT the
    /// same code path: no NaN canonicalization, no signed-zero tie-break
    /// special case, just a plain conditional select. Since IEEE-754 `<`
    /// is always `false` when either operand is NaN, `pmin` returns `a`
    /// (the FIRST operand) unchanged whenever either operand is NaN, NOT
    /// a canonicalized NaN result the way `MinF64x2` would produce --
    /// same first-operand-wins-on-NaN discipline as `f32x4.pmin`, the
    /// highest-risk correctness area in the `f32x4` PR this mirrors:
    /// copying `MinF64x2`'s NaN-canonicalization logic here would be
    /// WRONG.
    PminF64x2,
    /// `f64x2.pmax` (SIMD widen PR35) -- same "pseudo-max" shape as
    /// [`Self::PminF64x2`] above, symmetric formula: pop two `v128`s
    /// (`a` then `b`), compute each of the 2 lanes as `a < b ? b : a`
    /// using IEEE-754 `<` DIRECTLY, push one `v128`. Direct 2-lane
    /// mirror of [`Self::PmaxF32x4`]. Since `a < b` is always `false`
    /// when either operand is NaN, `pmax` also returns `a` (the FIRST
    /// operand) unchanged whenever either operand is NaN -- same
    /// first-operand-wins-on-NaN discipline as `pmin`, NOT
    /// [`Self::MaxF64x2`]'s NaN-canonicalizing behavior. Deliberately NOT
    /// implemented by reusing `MaxF64x2`'s NaN logic.
    PmaxF64x2,
}

/// One entry in the SIMD opcode table: everything a consumer needs to
/// decode and execute one `0xFD`-prefixed instruction in this first slice.
#[derive(Debug, Clone, Copy)]
pub struct SimdOpInfo {
    /// The canonical text name, e.g. `"i32x4.add"`.
    pub name: &'static str,
    /// The LEB128-encoded sub-opcode value (the integer immediately after
    /// the `0xFD` prefix byte) -- `u32`, NOT `u8` (see this section's own
    /// doc comment for why).
    pub sub_opcode: u32,
    pub kind: SimdOpKind,
}

/// The SIMD opcodes this repo implements, verified against authoritative
/// sources (the SIMD proposal's `BinarySIMD.md`, cross-checked against
/// the W3C core spec for the first 4) -- not guessed or reconstructed
/// from memory. `i32x4.extract_lane` is the one addition beyond the
/// original 4-opcode spec scope, added because it's the only way to
/// observe a `v128` result as a plain scalar -- see
/// `SimdOpKind::ExtractLane`'s own doc comment. The `i32x4.mul`/`neg`/
/// `sub` and full comparison family (`ne`/`lt_s`/`lt_u`/`le_s`/`le_u`/
/// `gt_s`/`gt_u`/`ge_s`/`ge_u`) widen this first slice to unblock the
/// real, pinned-commit `simd_i32x4_arith.wast`/`simd_i32x4_cmp.wast`
/// corpus files -- their exact sub-opcode bytes were fetched live from
/// `BinarySIMD.md` and cross-checked against the already-implemented
/// `i32x4.eq`/`i32x4.add` entries below (both matched exactly), same
/// verification discipline as the original 5. `i32x4.abs`/`min_s`/
/// `min_u`/`max_s`/`max_u` widen it further to unblock
/// `simd_i32x4_arith2.wast`, same verification discipline.
/// `i32x4.extadd_pairwise_i16x8_s`/`_u`, `i32x4.dot_i16x8_s`, and
/// `i32x4.extmul_low`/`high_i16x8_s`/`_u` widen it once more to unblock
/// `simd_i32x4_extadd_pairwise_i16x8.wast`/`simd_i32x4_dot_i16x8.wast`/
/// `simd_i32x4_extmul_i16x8.wast` -- the first opcodes in this table
/// whose INPUT lane width (16-bit `i16x8`) differs from their OUTPUT lane
/// width (32-bit `i32x4`), same live-fetched-and-cross-checked
/// verification discipline as every widening above.
/// `i8x16.add`/`sub`/`neg` are this table's first entries for the
/// `i8x16` lane width -- a brand-new "first slice" (following the same
/// pattern `i32x4` itself started with), not a widening of an existing
/// lane width. Each sub-opcode byte fetched live from `BinarySIMD.md`
/// and cross-checked against the already-implemented `i32x4.add`
/// (`0xAE`)/`i32x4.abs` (`0xA0`) entries (both matched exactly), same
/// discipline as every prior addition.
/// `i16x8.add`/`sub`/`mul`/`neg` are this table's first entries where
/// `i16x8` is a PRIMARY lane width (produces `i16x8` results, not just
/// read as a widening-op's input like `ExtaddPairwiseI16x8S`/etc.
/// above) -- another brand-new "first slice", same discipline. Each
/// sub-opcode byte fetched live from `BinarySIMD.md` and cross-checked
/// against the already-implemented `i32x4.add` (`0xAE`)/`i8x16.add`
/// (`0x6E`) entries (both matched exactly).
/// `i16x8.eq`/`ne`/`lt_s`/`lt_u`/`gt_s`/`gt_u`/`le_s`/`le_u`/`ge_s`/`ge_u`
/// widen `i16x8` further with its own comparison family, closing the gap
/// left when `i16x8.add`/`sub`/`mul`/`neg` landed without one (unlike
/// `i32x4`, which got arith+cmp together) -- same boolean-mask
/// convention and signed/unsigned split as `i32x4`'s own comparison
/// family, just at the narrower lane width. Each sub-opcode byte fetched
/// live from `BinarySIMD.md` and cross-checked against the already-
/// implemented `i16x8.add` (`0x8E`)/`i32x4.eq` (`0x37`) entries (both
/// matched exactly).
/// `i8x16.eq`/`ne`/`lt_s`/`lt_u`/`gt_s`/`gt_u`/`le_s`/`le_u`/`ge_s`/`ge_u`
/// close the same gap for `i8x16` -- it had arith (`add`/`sub`/`neg`) but
/// no comparison family until now, mirroring `i16x8`'s own pre-widening
/// state. Same boolean-mask convention and signed/unsigned split, just
/// at `i8x16`'s narrower lane width. Each sub-opcode byte fetched live
/// from `BinarySIMD.md` and cross-checked against the already-
/// implemented `i8x16.add` (`0x6E`)/`i16x8.eq` (`0x2D`) entries (both
/// matched exactly).
/// `i8x16.abs`/`popcnt`/`min_s`/`min_u`/`max_s`/`max_u`/`avgr_u` -- the
/// "arith2" family, mirroring `i32x4`'s own `abs`/`min_s`/`min_u`/
/// `max_s`/`max_u` widening, plus two op SHAPES with no `i32x4`/`i16x8`
/// precedent in this table: `popcnt` (lane-wise Hamming weight) and
/// `avgr_u` (lane-wise unsigned rounding average). Each sub-opcode byte
/// fetched live from `BinarySIMD.md` and cross-checked against the
/// already-implemented `i8x16.add` (`0x6E`)/`i8x16.neg` (`0x61`)/
/// `i8x16.sub` (`0x71`) entries (all three matched exactly).
/// `i16x8.abs`/`min_s`/`min_u`/`max_s`/`max_u`/`avgr_u` closes the same
/// "arith2" gap for `i16x8` that PR8 just closed for `i8x16` (no
/// `i16x8.popcnt` -- WASM SIMD only defines `popcnt` for `i8x16`). All
/// six sub-opcodes are >= 128 (2-byte LEB128), same shape as `i16x8`'s
/// own `add`/`sub`/`mul`/`neg`, unlike `i8x16`'s own arith2 family
/// (all < 128). Each sub-opcode byte fetched live from `BinarySIMD.md`
/// and cross-checked against the already-implemented `i16x8.neg`
/// (`0x81`)/`add` (`0x8E`)/`sub` (`0x91`)/`mul` (`0x95`) entries (all
/// four matched exactly).
/// `i16x8.extadd_pairwise_i8x16_s`/`_u`/`extmul_low`/`high_i8x16_s`/`_u`
/// mirrors the already-implemented `i32x4`-from-`i16x8` widening family,
/// one lane width down -- closing the last remaining gap between
/// `i16x8` and `i8x16`'s coverage. Unlike that family, there is no
/// `i16x8.dot_i8x16_s` -- WASM SIMD does not define a dot-product for
/// this lane-width pair. Each sub-opcode byte fetched live from
/// `BinarySIMD.md` and cross-checked against the already-implemented
/// `i8x16.add` (`0x6E`)/`i16x8.mul` (`0x95`)/`i16x8.avgr_u` (`0x9B`)/
/// `i32x4.dot_i16x8_s` (`0xBA`)/`i8x16.popcnt` (`0x62`)/
/// `i32x4.extadd_pairwise_i16x8_s` (`0x7E`) entries (all six matched
/// exactly).
/// `v128.not`/`and`/`andnot`/`or`/`xor`/`bitselect` -- the SIMD bitwise
/// family, lane-width-agnostic (the result never depends on how the
/// bits are interpreted as lanes). Closes the gap between the narrow
/// per-lane-width arithmetic families done so far (PR1-10) and the far
/// more universally-used masking/blending idioms every real SIMD
/// program relies on. `bitselect` is the first TERNARY SIMD op in this
/// table (pops three `v128`s, pushes one). Each sub-opcode byte fetched
/// live from `BinarySIMD.md` and cross-checked against the
/// already-implemented `i8x16.add` (`0x6E`)/`i32x4.add` (`0xAE`)
/// entries (both matched exactly).
/// `i8x16.swizzle`/`extract_lane_s`/`extract_lane_u`/`replace_lane`
/// (SIMD widen PR18) fill in the `0x0E`/`0x15`-`0x17` gap left inside
/// the already-implemented `0x0C`-`0x22` const/splat/extract_lane
/// encoding run -- `swizzle` reuses the plain BINARY `v128,v128->v128`
/// shape (like `i8x16.add`); `extract_lane_s`/`_u` reuse
/// `i32x4.extract_lane`'s "v128 + lane immediate -> i32" shape, just at
/// `i8x16`'s 0-15 lane range with a genuine signed/unsigned split (the
/// first `extract_lane` family member to need one); `replace_lane` is
/// a brand-new shape, combining a lane immediate with a mixed-type
/// (`v128`, `i32`) binary pop that produces a `v128` -- see
/// `SimdOpKind::ReplaceLaneI8x16`'s own doc comment. Each sub-opcode
/// byte fetched live from `BinarySIMD.md` and cross-checked against
/// the already-implemented `i32x4.extract_lane` (`0x1B`)/`i8x16.eq`
/// (`0x23`) entries, which sit exactly one past this run's own end
/// (both matched exactly, confirming the whole `0x0C`-`0x23` run is
/// contiguous and self-consistent).
/// `i32x4.trunc_sat_f32x4_s`/`_u`/`f32x4.convert_i32x4_s`/`_u` (SIMD
/// widen PR20) are this table's first `i32x4`<->`f32x4` CONVERSION
/// ops -- unlike every prior `f32x4` addition (PR17's splats, PR19's
/// abs/mul/min), which stayed within `f32x4` the whole way, these
/// change lane TYPE, not just value, matching the scalar `0xFC`-
/// prefixed `trunc_sat`/`convert` conversions this crate already
/// implements for non-SIMD MVP opcodes (see `wasm-execution`'s `0xFC`
/// handler). `trunc_sat_f32x4_s`/`_u` NEVER TRAP (NaN saturates to 0,
/// out-of-range saturates to the target bound) -- deliberately NOT the
/// same trapping behavior as this table's scalar `i32.trunc_f32_s`/
/// `_u` MVP opcodes. Each sub-opcode byte fetched live from
/// `BinarySIMD.md` and cross-checked against the already-implemented
/// `f32x4.abs` (`0xE0`)/`f32x4.min` (`0xE8`) entries (both matched
/// exactly). The 4 new sub-opcodes (`0xF8`-`0xFB`) are themselves a
/// contiguous, self-consistent run, though not adjacent to
/// `f32x4.min`'s `0xE8` -- the SIMD proposal's numbering leaves a gap
/// (`extend_low`/`high`, `narrow`, etc.) for opcodes this crate
/// doesn't implement yet.
/// `i64x2.extmul_low`/`high_i32x4_s`/`_u` (SIMD widen PR21) complete the
/// third and final rung of the "extmul" widening-multiply family this
/// table already implements twice over: `i16x8.extmul_low`/
/// `high_i8x16_s`/`_u` (`i8x16` -> `i16x8`) and `i32x4.extmul_low`/
/// `high_i16x8_s`/`_u` (`i16x8` -> `i32x4`). This is the `i32x4` ->
/// `i64x2` rung, same narrow-input/wide-output BINARY shape one lane
/// width up. Each sub-opcode byte fetched live from `BinarySIMD.md` and
/// cross-checked against the already-implemented `i32x4.extmul_low_i16x8_s`
/// (`0xBC`)/`i64x2.abs` (`0xC0`)/`i64x2.ge_s` (`0xDB`) entries (all three
/// matched exactly, confirming `0xDC`-`0xDF` sits immediately past
/// `i64x2`'s own comparison family with no gap).
/// `f32x4.neg`/`sqrt`/`add`/`sub`/`div` (SIMD widen PR29, task #202-204)
/// close the last remaining gap in `f32x4`'s core arithmetic family --
/// `abs`/`mul`/`min` landed in PR19, leaving `neg`/`sqrt`/`add`/`sub`/
/// `div`/`max`/`pmin`/`pmax` (this PR covers the first 5; `max`/`pmin`/
/// `pmax` landed later in SIMD widen PR34, task #217-219). Each sub-opcode byte fetched live from
/// `BinarySIMD.md` and cross-checked against the already-implemented
/// `f32x4.abs` (`0xE0`)/`f32x4.mul` (`0xE6`)/`f32x4.min` (`0xE8`)
/// entries: `neg` (`0xE1`) sits immediately past `abs`, `sqrt` (`0xE3`)
/// two past that (`0xE2` is unassigned in the SIMD proposal's own binary
/// encoding -- confirmed by its absence from `BinarySIMD.md` entirely,
/// not a placeholder for a future op this crate is skipping), `add`/
/// `sub` (`0xE4`/`0xE5`) sit immediately before `mul`, and `div`
/// (`0xE7`) sits immediately between `mul` and `min` -- all five
/// confirmed free of collision with every existing `SIMD_OPS` entry.
/// This PR also vendors `simd_f32x4_arith.wast`, the single biggest
/// directive-count win in this campaign so far -- see
/// `code/packages/rust/wasm-conformance/tests/fixtures/
/// fetch_testsuite.py`.
pub static SIMD_OPS: &[SimdOpInfo] = &[
    SimdOpInfo { name: "v128.const", sub_opcode: 0x0C, kind: SimdOpKind::Const },
    SimdOpInfo { name: "i32x4.extract_lane", sub_opcode: 0x1B, kind: SimdOpKind::ExtractLane },
    SimdOpInfo { name: "i32x4.splat", sub_opcode: 0x11, kind: SimdOpKind::Splat },
    SimdOpInfo { name: "i32x4.eq", sub_opcode: 0x37, kind: SimdOpKind::Eq },
    SimdOpInfo { name: "i32x4.ne", sub_opcode: 0x38, kind: SimdOpKind::Ne },
    SimdOpInfo { name: "i32x4.lt_s", sub_opcode: 0x39, kind: SimdOpKind::LtS },
    SimdOpInfo { name: "i32x4.lt_u", sub_opcode: 0x3A, kind: SimdOpKind::LtU },
    SimdOpInfo { name: "i32x4.gt_s", sub_opcode: 0x3B, kind: SimdOpKind::GtS },
    SimdOpInfo { name: "i32x4.gt_u", sub_opcode: 0x3C, kind: SimdOpKind::GtU },
    SimdOpInfo { name: "i32x4.le_s", sub_opcode: 0x3D, kind: SimdOpKind::LeS },
    SimdOpInfo { name: "i32x4.le_u", sub_opcode: 0x3E, kind: SimdOpKind::LeU },
    SimdOpInfo { name: "i32x4.ge_s", sub_opcode: 0x3F, kind: SimdOpKind::GeS },
    SimdOpInfo { name: "i32x4.ge_u", sub_opcode: 0x40, kind: SimdOpKind::GeU },
    SimdOpInfo { name: "i32x4.abs", sub_opcode: 0xA0, kind: SimdOpKind::Abs },
    SimdOpInfo { name: "i32x4.neg", sub_opcode: 0xA1, kind: SimdOpKind::Neg },
    SimdOpInfo { name: "i32x4.add", sub_opcode: 0xAE, kind: SimdOpKind::Add },
    SimdOpInfo { name: "i32x4.sub", sub_opcode: 0xB1, kind: SimdOpKind::Sub },
    SimdOpInfo { name: "i32x4.mul", sub_opcode: 0xB5, kind: SimdOpKind::Mul },
    SimdOpInfo { name: "i32x4.min_s", sub_opcode: 0xB6, kind: SimdOpKind::MinS },
    SimdOpInfo { name: "i32x4.min_u", sub_opcode: 0xB7, kind: SimdOpKind::MinU },
    SimdOpInfo { name: "i32x4.max_s", sub_opcode: 0xB8, kind: SimdOpKind::MaxS },
    SimdOpInfo { name: "i32x4.max_u", sub_opcode: 0xB9, kind: SimdOpKind::MaxU },
    SimdOpInfo { name: "i32x4.extadd_pairwise_i16x8_s", sub_opcode: 0x7E, kind: SimdOpKind::ExtaddPairwiseI16x8S },
    SimdOpInfo { name: "i32x4.extadd_pairwise_i16x8_u", sub_opcode: 0x7F, kind: SimdOpKind::ExtaddPairwiseI16x8U },
    SimdOpInfo { name: "i32x4.dot_i16x8_s", sub_opcode: 0xBA, kind: SimdOpKind::DotI16x8S },
    SimdOpInfo { name: "i32x4.extmul_low_i16x8_s", sub_opcode: 0xBC, kind: SimdOpKind::ExtmulLowI16x8S },
    SimdOpInfo { name: "i32x4.extmul_high_i16x8_s", sub_opcode: 0xBD, kind: SimdOpKind::ExtmulHighI16x8S },
    SimdOpInfo { name: "i32x4.extmul_low_i16x8_u", sub_opcode: 0xBE, kind: SimdOpKind::ExtmulLowI16x8U },
    SimdOpInfo { name: "i32x4.extmul_high_i16x8_u", sub_opcode: 0xBF, kind: SimdOpKind::ExtmulHighI16x8U },
    SimdOpInfo { name: "i8x16.neg", sub_opcode: 0x61, kind: SimdOpKind::NegI8x16 },
    SimdOpInfo { name: "i8x16.add", sub_opcode: 0x6E, kind: SimdOpKind::AddI8x16 },
    SimdOpInfo { name: "i8x16.sub", sub_opcode: 0x71, kind: SimdOpKind::SubI8x16 },
    SimdOpInfo { name: "i16x8.neg", sub_opcode: 0x81, kind: SimdOpKind::NegI16x8 },
    SimdOpInfo { name: "i16x8.add", sub_opcode: 0x8E, kind: SimdOpKind::AddI16x8 },
    SimdOpInfo { name: "i16x8.sub", sub_opcode: 0x91, kind: SimdOpKind::SubI16x8 },
    SimdOpInfo { name: "i16x8.mul", sub_opcode: 0x95, kind: SimdOpKind::MulI16x8 },
    SimdOpInfo { name: "i16x8.eq", sub_opcode: 0x2D, kind: SimdOpKind::EqI16x8 },
    SimdOpInfo { name: "i16x8.ne", sub_opcode: 0x2E, kind: SimdOpKind::NeI16x8 },
    SimdOpInfo { name: "i16x8.lt_s", sub_opcode: 0x2F, kind: SimdOpKind::LtSI16x8 },
    SimdOpInfo { name: "i16x8.lt_u", sub_opcode: 0x30, kind: SimdOpKind::LtUI16x8 },
    SimdOpInfo { name: "i16x8.gt_s", sub_opcode: 0x31, kind: SimdOpKind::GtSI16x8 },
    SimdOpInfo { name: "i16x8.gt_u", sub_opcode: 0x32, kind: SimdOpKind::GtUI16x8 },
    SimdOpInfo { name: "i16x8.le_s", sub_opcode: 0x33, kind: SimdOpKind::LeSI16x8 },
    SimdOpInfo { name: "i16x8.le_u", sub_opcode: 0x34, kind: SimdOpKind::LeUI16x8 },
    SimdOpInfo { name: "i16x8.ge_s", sub_opcode: 0x35, kind: SimdOpKind::GeSI16x8 },
    SimdOpInfo { name: "i16x8.ge_u", sub_opcode: 0x36, kind: SimdOpKind::GeUI16x8 },
    SimdOpInfo { name: "i8x16.eq", sub_opcode: 0x23, kind: SimdOpKind::EqI8x16 },
    SimdOpInfo { name: "i8x16.ne", sub_opcode: 0x24, kind: SimdOpKind::NeI8x16 },
    SimdOpInfo { name: "i8x16.lt_s", sub_opcode: 0x25, kind: SimdOpKind::LtSI8x16 },
    SimdOpInfo { name: "i8x16.lt_u", sub_opcode: 0x26, kind: SimdOpKind::LtUI8x16 },
    SimdOpInfo { name: "i8x16.gt_s", sub_opcode: 0x27, kind: SimdOpKind::GtSI8x16 },
    SimdOpInfo { name: "i8x16.gt_u", sub_opcode: 0x28, kind: SimdOpKind::GtUI8x16 },
    SimdOpInfo { name: "i8x16.le_s", sub_opcode: 0x29, kind: SimdOpKind::LeSI8x16 },
    SimdOpInfo { name: "i8x16.le_u", sub_opcode: 0x2A, kind: SimdOpKind::LeUI8x16 },
    SimdOpInfo { name: "i8x16.ge_s", sub_opcode: 0x2B, kind: SimdOpKind::GeSI8x16 },
    SimdOpInfo { name: "i8x16.ge_u", sub_opcode: 0x2C, kind: SimdOpKind::GeUI8x16 },
    SimdOpInfo { name: "i8x16.abs", sub_opcode: 0x60, kind: SimdOpKind::AbsI8x16 },
    SimdOpInfo { name: "i8x16.popcnt", sub_opcode: 0x62, kind: SimdOpKind::PopcntI8x16 },
    SimdOpInfo { name: "i8x16.min_s", sub_opcode: 0x76, kind: SimdOpKind::MinSI8x16 },
    SimdOpInfo { name: "i8x16.min_u", sub_opcode: 0x77, kind: SimdOpKind::MinUI8x16 },
    SimdOpInfo { name: "i8x16.max_s", sub_opcode: 0x78, kind: SimdOpKind::MaxSI8x16 },
    SimdOpInfo { name: "i8x16.max_u", sub_opcode: 0x79, kind: SimdOpKind::MaxUI8x16 },
    SimdOpInfo { name: "i8x16.avgr_u", sub_opcode: 0x7B, kind: SimdOpKind::AvgrUI8x16 },
    SimdOpInfo { name: "i16x8.abs", sub_opcode: 0x80, kind: SimdOpKind::AbsI16x8 },
    SimdOpInfo { name: "i16x8.min_s", sub_opcode: 0x96, kind: SimdOpKind::MinSI16x8 },
    SimdOpInfo { name: "i16x8.min_u", sub_opcode: 0x97, kind: SimdOpKind::MinUI16x8 },
    SimdOpInfo { name: "i16x8.max_s", sub_opcode: 0x98, kind: SimdOpKind::MaxSI16x8 },
    SimdOpInfo { name: "i16x8.max_u", sub_opcode: 0x99, kind: SimdOpKind::MaxUI16x8 },
    SimdOpInfo { name: "i16x8.avgr_u", sub_opcode: 0x9B, kind: SimdOpKind::AvgrUI16x8 },
    SimdOpInfo { name: "i16x8.extadd_pairwise_i8x16_s", sub_opcode: 0x7C, kind: SimdOpKind::ExtaddPairwiseI8x16S },
    SimdOpInfo { name: "i16x8.extadd_pairwise_i8x16_u", sub_opcode: 0x7D, kind: SimdOpKind::ExtaddPairwiseI8x16U },
    SimdOpInfo { name: "i16x8.extmul_low_i8x16_s", sub_opcode: 0x9C, kind: SimdOpKind::ExtmulLowI8x16S },
    SimdOpInfo { name: "i16x8.extmul_high_i8x16_s", sub_opcode: 0x9D, kind: SimdOpKind::ExtmulHighI8x16S },
    SimdOpInfo { name: "i16x8.extmul_low_i8x16_u", sub_opcode: 0x9E, kind: SimdOpKind::ExtmulLowI8x16U },
    SimdOpInfo { name: "i16x8.extmul_high_i8x16_u", sub_opcode: 0x9F, kind: SimdOpKind::ExtmulHighI8x16U },
    SimdOpInfo { name: "v128.not", sub_opcode: 0x4D, kind: SimdOpKind::Not },
    SimdOpInfo { name: "v128.and", sub_opcode: 0x4E, kind: SimdOpKind::And },
    SimdOpInfo { name: "v128.andnot", sub_opcode: 0x4F, kind: SimdOpKind::AndNot },
    SimdOpInfo { name: "v128.or", sub_opcode: 0x50, kind: SimdOpKind::Or },
    SimdOpInfo { name: "v128.xor", sub_opcode: 0x51, kind: SimdOpKind::Xor },
    SimdOpInfo { name: "v128.bitselect", sub_opcode: 0x52, kind: SimdOpKind::Bitselect },
    SimdOpInfo { name: "v128.any_true", sub_opcode: 0x53, kind: SimdOpKind::AnyTrue },
    SimdOpInfo { name: "i8x16.all_true", sub_opcode: 0x63, kind: SimdOpKind::AllTrueI8x16 },
    SimdOpInfo { name: "i8x16.bitmask", sub_opcode: 0x64, kind: SimdOpKind::BitmaskI8x16 },
    SimdOpInfo { name: "i16x8.all_true", sub_opcode: 0x83, kind: SimdOpKind::AllTrueI16x8 },
    SimdOpInfo { name: "i16x8.bitmask", sub_opcode: 0x84, kind: SimdOpKind::BitmaskI16x8 },
    SimdOpInfo { name: "i32x4.all_true", sub_opcode: 0xA3, kind: SimdOpKind::AllTrueI32x4 },
    SimdOpInfo { name: "i32x4.bitmask", sub_opcode: 0xA4, kind: SimdOpKind::BitmaskI32x4 },
    SimdOpInfo { name: "i64x2.all_true", sub_opcode: 0xC3, kind: SimdOpKind::AllTrueI64x2 },
    SimdOpInfo { name: "i64x2.bitmask", sub_opcode: 0xC4, kind: SimdOpKind::BitmaskI64x2 },
    SimdOpInfo { name: "i64x2.abs", sub_opcode: 0xC0, kind: SimdOpKind::AbsI64x2 },
    SimdOpInfo { name: "i64x2.neg", sub_opcode: 0xC1, kind: SimdOpKind::NegI64x2 },
    SimdOpInfo { name: "i64x2.add", sub_opcode: 0xCE, kind: SimdOpKind::AddI64x2 },
    SimdOpInfo { name: "i64x2.sub", sub_opcode: 0xD1, kind: SimdOpKind::SubI64x2 },
    SimdOpInfo { name: "i64x2.mul", sub_opcode: 0xD5, kind: SimdOpKind::MulI64x2 },
    SimdOpInfo { name: "i64x2.eq", sub_opcode: 0xD6, kind: SimdOpKind::EqI64x2 },
    SimdOpInfo { name: "i64x2.ne", sub_opcode: 0xD7, kind: SimdOpKind::NeI64x2 },
    SimdOpInfo { name: "i64x2.lt_s", sub_opcode: 0xD8, kind: SimdOpKind::LtSI64x2 },
    SimdOpInfo { name: "i64x2.gt_s", sub_opcode: 0xD9, kind: SimdOpKind::GtSI64x2 },
    SimdOpInfo { name: "i64x2.le_s", sub_opcode: 0xDA, kind: SimdOpKind::LeSI64x2 },
    SimdOpInfo { name: "i64x2.ge_s", sub_opcode: 0xDB, kind: SimdOpKind::GeSI64x2 },
    SimdOpInfo { name: "i8x16.shl", sub_opcode: 0x6B, kind: SimdOpKind::ShlI8x16 },
    SimdOpInfo { name: "i8x16.shr_s", sub_opcode: 0x6C, kind: SimdOpKind::ShrSI8x16 },
    SimdOpInfo { name: "i8x16.shr_u", sub_opcode: 0x6D, kind: SimdOpKind::ShrUI8x16 },
    SimdOpInfo { name: "i16x8.shl", sub_opcode: 0x8B, kind: SimdOpKind::ShlI16x8 },
    SimdOpInfo { name: "i16x8.shr_s", sub_opcode: 0x8C, kind: SimdOpKind::ShrSI16x8 },
    SimdOpInfo { name: "i16x8.shr_u", sub_opcode: 0x8D, kind: SimdOpKind::ShrUI16x8 },
    SimdOpInfo { name: "i32x4.shl", sub_opcode: 0xAB, kind: SimdOpKind::ShlI32x4 },
    SimdOpInfo { name: "i32x4.shr_s", sub_opcode: 0xAC, kind: SimdOpKind::ShrSI32x4 },
    SimdOpInfo { name: "i32x4.shr_u", sub_opcode: 0xAD, kind: SimdOpKind::ShrUI32x4 },
    SimdOpInfo { name: "i64x2.shl", sub_opcode: 0xCB, kind: SimdOpKind::ShlI64x2 },
    SimdOpInfo { name: "i64x2.shr_s", sub_opcode: 0xCC, kind: SimdOpKind::ShrSI64x2 },
    SimdOpInfo { name: "i64x2.shr_u", sub_opcode: 0xCD, kind: SimdOpKind::ShrUI64x2 },
    SimdOpInfo { name: "v128.load", sub_opcode: 0x00, kind: SimdOpKind::Load },
    SimdOpInfo { name: "v128.store", sub_opcode: 0x0B, kind: SimdOpKind::Store },
    SimdOpInfo { name: "i8x16.splat", sub_opcode: 0x0F, kind: SimdOpKind::SplatI8x16 },
    SimdOpInfo { name: "i16x8.splat", sub_opcode: 0x10, kind: SimdOpKind::SplatI16x8 },
    SimdOpInfo { name: "i64x2.splat", sub_opcode: 0x12, kind: SimdOpKind::SplatI64x2 },
    SimdOpInfo { name: "f32x4.splat", sub_opcode: 0x13, kind: SimdOpKind::SplatF32x4 },
    SimdOpInfo { name: "f64x2.splat", sub_opcode: 0x14, kind: SimdOpKind::SplatF64x2 },
    SimdOpInfo { name: "i8x16.swizzle", sub_opcode: 0x0E, kind: SimdOpKind::Swizzle },
    SimdOpInfo { name: "i8x16.extract_lane_s", sub_opcode: 0x15, kind: SimdOpKind::ExtractLaneI8x16S },
    SimdOpInfo { name: "i8x16.extract_lane_u", sub_opcode: 0x16, kind: SimdOpKind::ExtractLaneI8x16U },
    SimdOpInfo { name: "i8x16.replace_lane", sub_opcode: 0x17, kind: SimdOpKind::ReplaceLaneI8x16 },
    // ── SIMD widen PR37: the remaining extract_lane/replace_lane family
    // members across i16x8/i32x4/i64x2/f32x4/f64x2 (i8x16's trio and
    // i32x4.extract_lane above already existed from PR1b-2/PR18). Each
    // sub-opcode byte fetched live from BinarySIMD.md and cross-checked
    // against the already-implemented i8x16.extract_lane_s (0x15)/
    // extract_lane_u (0x16)/replace_lane (0x17) and i32x4.extract_lane
    // (0x1B) entries above -- confirming the whole 0x15-0x22 lane-op run
    // is contiguous and self-consistent.
    SimdOpInfo { name: "i16x8.extract_lane_s", sub_opcode: 0x18, kind: SimdOpKind::ExtractLaneI16x8S },
    SimdOpInfo { name: "i16x8.extract_lane_u", sub_opcode: 0x19, kind: SimdOpKind::ExtractLaneI16x8U },
    SimdOpInfo { name: "i16x8.replace_lane", sub_opcode: 0x1A, kind: SimdOpKind::ReplaceLaneI16x8 },
    SimdOpInfo { name: "i32x4.replace_lane", sub_opcode: 0x1C, kind: SimdOpKind::ReplaceLaneI32x4 },
    SimdOpInfo { name: "i64x2.extract_lane", sub_opcode: 0x1D, kind: SimdOpKind::ExtractLaneI64x2 },
    SimdOpInfo { name: "i64x2.replace_lane", sub_opcode: 0x1E, kind: SimdOpKind::ReplaceLaneI64x2 },
    SimdOpInfo { name: "f32x4.extract_lane", sub_opcode: 0x1F, kind: SimdOpKind::ExtractLaneF32x4 },
    SimdOpInfo { name: "f32x4.replace_lane", sub_opcode: 0x20, kind: SimdOpKind::ReplaceLaneF32x4 },
    SimdOpInfo { name: "f64x2.extract_lane", sub_opcode: 0x21, kind: SimdOpKind::ExtractLaneF64x2 },
    SimdOpInfo { name: "f64x2.replace_lane", sub_opcode: 0x22, kind: SimdOpKind::ReplaceLaneF64x2 },
    SimdOpInfo { name: "f32x4.abs", sub_opcode: 0xE0, kind: SimdOpKind::AbsF32x4 },
    SimdOpInfo { name: "f32x4.mul", sub_opcode: 0xE6, kind: SimdOpKind::MulF32x4 },
    SimdOpInfo { name: "f32x4.min", sub_opcode: 0xE8, kind: SimdOpKind::MinF32x4 },
    // SIMD widen PR29 (task #202-204): f32x4.neg/sqrt/add/sub/div --
    // closes the last remaining gap in f32x4's core arithmetic family
    // (abs/mul/min landed in PR19). Each sub-opcode byte fetched live
    // from BinarySIMD.md and cross-checked against the already-
    // implemented f32x4.abs (0xE0)/f32x4.mul (0xE6)/f32x4.min (0xE8)
    // entries (all matched exactly) -- see this table's own doc comment
    // above for the full gap analysis. Also vendors
    // simd_f32x4_arith.wast, the single biggest directive-count win in
    // this campaign so far.
    SimdOpInfo { name: "f32x4.neg", sub_opcode: 0xE1, kind: SimdOpKind::NegF32x4 },
    SimdOpInfo { name: "f32x4.sqrt", sub_opcode: 0xE3, kind: SimdOpKind::SqrtF32x4 },
    SimdOpInfo { name: "f32x4.add", sub_opcode: 0xE4, kind: SimdOpKind::AddF32x4 },
    SimdOpInfo { name: "f32x4.sub", sub_opcode: 0xE5, kind: SimdOpKind::SubF32x4 },
    SimdOpInfo { name: "f32x4.div", sub_opcode: 0xE7, kind: SimdOpKind::DivF32x4 },
    // SIMD widen PR30 (task #205-207): f32x4.eq/ne/lt/gt/le/ge (0x41-0x46)
    // -- the f32x4 comparison family, identical lane-wise boolean-mask
    // shape as the already-implemented i32x4/i16x8/i8x16/i64x2
    // comparison families (see e.g. `SimdOpKind::Eq`'s own doc comment),
    // just over 4 `f32` lanes with native IEEE-754 float comparison
    // operators instead of integer comparison -- no signed/unsigned
    // split (floats have none), and NaN operands make every comparison
    // false except `ne` (true), per IEEE-754 unordered-comparison
    // semantics that Rust's native `f32` operators already implement
    // correctly. Each sub-opcode byte fetched live from BinarySIMD.md
    // and cross-checked against every existing `SIMD_OPS` entry: 0x41-
    // 0x46 sit in the SIMD sub-opcode space behind the `0xFD` prefix (an
    // entirely separate byte space from the single-byte `OPCODES` table,
    // so no collision with e.g. `i32.const`'s unrelated `0x41` there,
    // and no collision with `ATOMIC_OPS`'s own unrelated `0x41`-`0x46`
    // behind the `0xFE` prefix either) and confirmed free of collision
    // with every existing `SIMD_OPS` entry -- the closest neighbors are
    // `i32x4.eq`..`i32x4.ge_u` at `0x37`-`0x40` (just below) and
    // `v128.not` at `0x4D` (just above), leaving 0x41-0x46 genuinely
    // open. This PR also vendors `simd_f32x4_cmp.wast`, the single
    // biggest directive-count win in this campaign so far -- see
    // `code/packages/rust/wasm-conformance/tests/fixtures/
    // fetch_testsuite.py`.
    SimdOpInfo { name: "f32x4.eq", sub_opcode: 0x41, kind: SimdOpKind::EqF32x4 },
    SimdOpInfo { name: "f32x4.ne", sub_opcode: 0x42, kind: SimdOpKind::NeF32x4 },
    SimdOpInfo { name: "f32x4.lt", sub_opcode: 0x43, kind: SimdOpKind::LtF32x4 },
    SimdOpInfo { name: "f32x4.gt", sub_opcode: 0x44, kind: SimdOpKind::GtF32x4 },
    SimdOpInfo { name: "f32x4.le", sub_opcode: 0x45, kind: SimdOpKind::LeF32x4 },
    SimdOpInfo { name: "f32x4.ge", sub_opcode: 0x46, kind: SimdOpKind::GeF32x4 },
    SimdOpInfo { name: "i32x4.trunc_sat_f32x4_s", sub_opcode: 0xF8, kind: SimdOpKind::TruncSatF32x4S },
    SimdOpInfo { name: "i32x4.trunc_sat_f32x4_u", sub_opcode: 0xF9, kind: SimdOpKind::TruncSatF32x4U },
    SimdOpInfo { name: "f32x4.convert_i32x4_s", sub_opcode: 0xFA, kind: SimdOpKind::ConvertI32x4S },
    SimdOpInfo { name: "f32x4.convert_i32x4_u", sub_opcode: 0xFB, kind: SimdOpKind::ConvertI32x4U },
    SimdOpInfo { name: "i64x2.extmul_low_i32x4_s", sub_opcode: 0xDC, kind: SimdOpKind::ExtmulLowI64x2S },
    SimdOpInfo { name: "i64x2.extmul_high_i32x4_s", sub_opcode: 0xDD, kind: SimdOpKind::ExtmulHighI64x2S },
    SimdOpInfo { name: "i64x2.extmul_low_i32x4_u", sub_opcode: 0xDE, kind: SimdOpKind::ExtmulLowI64x2U },
    SimdOpInfo { name: "i64x2.extmul_high_i32x4_u", sub_opcode: 0xDF, kind: SimdOpKind::ExtmulHighI64x2U },
    SimdOpInfo { name: "i16x8.q15mulr_sat_s", sub_opcode: 0x82, kind: SimdOpKind::Q15mulrSatI16x8S },
    // SIMD widen PR25 (task #190-192): i32x4.trunc_sat_f64x2_s_zero/
    // _u_zero -- the f64x2-source rung of the "_zero" trunc_sat family,
    // immediately past f32x4.convert_i32x4_u's 0xFB with no gap. Each
    // sub-opcode byte fetched live from BinarySIMD.md and cross-checked
    // against the already-implemented i32x4.trunc_sat_f32x4_s/_u (0xF8/
    // 0xF9)/f32x4.convert_i32x4_s/_u (0xFA/0xFB) entries (all four
    // matched exactly, confirming 0xFC/0xFD sit immediately past that
    // conversion family with no gap).
    SimdOpInfo { name: "i32x4.trunc_sat_f64x2_s_zero", sub_opcode: 0xFC, kind: SimdOpKind::TruncSatF64x2SZero },
    SimdOpInfo { name: "i32x4.trunc_sat_f64x2_u_zero", sub_opcode: 0xFD, kind: SimdOpKind::TruncSatF64x2UZero },
    // SIMD widen PR26 (task #193-195): i16x8.extend_low/high_i8x16_s/_u
    // (0x87/0x88/0x89/0x8A) and i32x4.extend_low/high_i16x8_s/_u
    // (0xA7/0xA8/0xA9/0xAA) -- the "extend" family, EXACTLY the
    // lane-selection + sign/zero-extend half of the already-implemented
    // ExtmulLowI8x16S/ExtmulHighI8x16S/etc. handlers, minus the multiply.
    // Each sub-opcode byte fetched live from BinarySIMD.md and
    // cross-checked against the already-implemented i16x8.extmul_low_i8x16_s
    // (0x9C)/i16x8.shl (0x8B)/i16x8.q15mulr_sat_s (0x82)/
    // i32x4.extmul_low_i16x8_s (0xBC)/i32x4.all_true (0xA3)/i32x4.shl
    // (0xAB) entries (all six matched exactly, confirming 0x87-0x8A and
    // 0xA7-0xAA are free gaps in their respective runs). This is one of
    // three PRs (this one, a future "narrow" PR, and a future
    // "promote/demote/convert_low" PR) needed to land all 16 opcodes the
    // upstream simd_conversions.wast corpus file bundles together in its
    // modules -- NO corpus vendoring happens until all 16 are in, since
    // that file can't be partially satisfied. This PR is opcode-only,
    // verified by unit tests.
    SimdOpInfo { name: "i16x8.extend_low_i8x16_s", sub_opcode: 0x87, kind: SimdOpKind::ExtendLowI8x16S },
    SimdOpInfo { name: "i16x8.extend_high_i8x16_s", sub_opcode: 0x88, kind: SimdOpKind::ExtendHighI8x16S },
    SimdOpInfo { name: "i16x8.extend_low_i8x16_u", sub_opcode: 0x89, kind: SimdOpKind::ExtendLowI8x16U },
    SimdOpInfo { name: "i16x8.extend_high_i8x16_u", sub_opcode: 0x8A, kind: SimdOpKind::ExtendHighI8x16U },
    SimdOpInfo { name: "i32x4.extend_low_i16x8_s", sub_opcode: 0xA7, kind: SimdOpKind::ExtendLowI16x8S },
    SimdOpInfo { name: "i32x4.extend_high_i16x8_s", sub_opcode: 0xA8, kind: SimdOpKind::ExtendHighI16x8S },
    SimdOpInfo { name: "i32x4.extend_low_i16x8_u", sub_opcode: 0xA9, kind: SimdOpKind::ExtendLowI16x8U },
    SimdOpInfo { name: "i32x4.extend_high_i16x8_u", sub_opcode: 0xAA, kind: SimdOpKind::ExtendHighI16x8U },
    // SIMD widen PR36 (task #223-225): i64x2.extend_low/high_i32x4_s/_u
    // (0xC7/0xC8/0xC9/0xCA) -- the THIRD and FINAL rung of the "extend"
    // family (i16x8-from-i8x16 and i32x4-from-i16x8 both landed in
    // PR26 above; this completes i64x2-from-i32x4), EXACTLY the
    // lane-selection + sign/zero-extend half of the already-implemented
    // ExtmulLowI64x2S/ExtmulHighI64x2S/etc. handlers, minus the multiply
    // -- same shape as PR26's own i16x8/i32x4 rungs, one lane width up.
    // Each sub-opcode byte fetched live from BinarySIMD.md and
    // cross-checked against every existing `SIMD_OPS` entry: the closest
    // neighbors are `i64x2.bitmask` at 0xC4 (just below) and
    // `i64x2.shl` at 0xCB (just above) -- 0xC7-0xCA sit in the gap
    // between them with no collision (0xC5/0xC6 remain unassigned/
    // unimplemented, not part of this PR). This PR also vendors
    // `simd_int_to_int_extend.wast`, the upstream corpus file dedicated
    // to the whole 3-rung extend family -- it exercises PR26's
    // already-implemented i16x8/i32x4 opcodes too, so a correct
    // implementation should pass 100% of the file, not just the new
    // i64x2 opcodes' directives.
    SimdOpInfo { name: "i64x2.extend_low_i32x4_s", sub_opcode: 0xC7, kind: SimdOpKind::ExtendLowI32x4S },
    SimdOpInfo { name: "i64x2.extend_high_i32x4_s", sub_opcode: 0xC8, kind: SimdOpKind::ExtendHighI32x4S },
    SimdOpInfo { name: "i64x2.extend_low_i32x4_u", sub_opcode: 0xC9, kind: SimdOpKind::ExtendLowI32x4U },
    SimdOpInfo { name: "i64x2.extend_high_i32x4_u", sub_opcode: 0xCA, kind: SimdOpKind::ExtendHighI32x4U },
    // SIMD widen PR27 (task #196-198): i8x16.narrow_i16x8_s/_u
    // (0x65/0x66) and i16x8.narrow_i32x4_s/_u (0x85/0x86) -- the
    // "narrow" family, the saturating-demote OPPOSITE of PR26's
    // "extend" family: BINARY (two v128 operands, not one), each
    // lane SATURATED (not wrapped) down to the narrower width and
    // concatenated (first operand -> low half, second operand -> high
    // half of the result). Each sub-opcode byte fetched live from
    // BinarySIMD.md (this is the SIMD sub-opcode space behind the
    // `0xFD` prefix -- an entirely separate byte space from the
    // single-byte `OPCODES` table above, so no collision is possible
    // with e.g. `f64x2.le`'s unrelated `0x65` there) and cross-checked
    // against the already-implemented `i8x16.bitmask` (0x64)/
    // `i16x8.all_true` (0x83)/`i16x8.bitmask` (0x84) `SIMD_OPS`
    // entries: 0x65/0x66 sit immediately past `i8x16.bitmask`'s 0x64
    // with no gap, and 0x85/0x86 sit immediately past
    // `i16x8.bitmask`'s 0x84 with no gap -- both confirmed free of
    // collision with every existing `SIMD_OPS` entry (PR26's 0x87-0x8A
    // and 0xA7-0xAA run don't overlap either). Second of three PRs
    // (extend done in PR26, narrow here, promote/demote/convert_low to
    // follow) needed to land all 16 opcodes the upstream
    // simd_conversions.wast corpus file bundles together -- still NO
    // corpus vendoring in this PR, opcode-only, verified by unit
    // tests.
    SimdOpInfo { name: "i8x16.narrow_i16x8_s", sub_opcode: 0x65, kind: SimdOpKind::NarrowI16x8S },
    SimdOpInfo { name: "i8x16.narrow_i16x8_u", sub_opcode: 0x66, kind: SimdOpKind::NarrowI16x8U },
    SimdOpInfo { name: "i16x8.narrow_i32x4_s", sub_opcode: 0x85, kind: SimdOpKind::NarrowI32x4S },
    SimdOpInfo { name: "i16x8.narrow_i32x4_u", sub_opcode: 0x86, kind: SimdOpKind::NarrowI32x4U },
    // SIMD widen PR28 (task #199-201): f32x4.demote_f64x2_zero (0x5E),
    // f64x2.promote_low_f32x4 (0x5F), f64x2.convert_low_i32x4_s (0xFE),
    // f64x2.convert_low_i32x4_u (0xFF) -- the "promote/demote/
    // convert_low" family, THIRD and FINAL of three PRs (extend done in
    // PR26, narrow done in PR27, this one) needed to land all 16
    // opcodes the upstream simd_conversions.wast corpus file bundles
    // together in its modules. Each sub-opcode byte fetched live from
    // BinarySIMD.md and cross-checked against this table's own already-
    // implemented neighbors: 0x5E/0x5F sit immediately past v128.xor's
    // (0x51) and v128.bitselect's (0x52) run and well clear of
    // i8x16.narrow_i16x8_s/_u's 0x65/0x66 just above, no collision;
    // 0xFE/0xFF sit immediately past i32x4.trunc_sat_f64x2_u_zero's
    // 0xFD (PR25) with no gap, no collision with any entry in this
    // table. With this PR landed, all 16 opcodes now exist, so
    // simd_conversions.wast is vendored for the first time -- see
    // `code/packages/rust/wasm-conformance/tests/fixtures/
    // fetch_testsuite.py`.
    SimdOpInfo { name: "f32x4.demote_f64x2_zero", sub_opcode: 0x5E, kind: SimdOpKind::DemoteF64x2Zero },
    SimdOpInfo { name: "f64x2.promote_low_f32x4", sub_opcode: 0x5F, kind: SimdOpKind::PromoteLowF32x4 },
    SimdOpInfo { name: "f64x2.convert_low_i32x4_s", sub_opcode: 0xFE, kind: SimdOpKind::ConvertLowI32x4S },
    SimdOpInfo { name: "f64x2.convert_low_i32x4_u", sub_opcode: 0xFF, kind: SimdOpKind::ConvertLowI32x4U },
    // SIMD widen PR31 (task #208-210): f64x2.neg/sqrt/add/sub/mul/div --
    // the f64x2 core arithmetic family, a direct structural mirror of
    // PR29's f32x4.neg/sqrt/add/sub/div, just at f64x2's 2-lane width,
    // plus `mul` (f32x4.mul already existed pre-PR29; f64x2.mul did not
    // exist yet, so it rides along here on the same binary-op
    // boilerplate as add/sub/div). Each sub-opcode byte fetched live
    // from BinarySIMD.md and cross-checked against every existing
    // `SIMD_OPS` entry: 0xEC (f64x2.abs, still unimplemented) precedes
    // this run, 0xED is `neg`, 0xEE (f64x2.sqrt's slot minus one) is
    // unassigned in the SIMD proposal's own binary encoding -- same gap
    // shape as f32x4's own 0xE2 gap between abs/neg and sqrt -- 0xEF is
    // `sqrt`, 0xF0-0xF3 are `add`/`sub`/`mul`/`div` in that order, and
    // 0xF4/0xF5 (f64x2.min/max, still unimplemented) sit immediately
    // past this run with no overlap. All six confirmed free of collision
    // with every existing `SIMD_OPS` entry. This PR also vendors
    // `simd_f64x2_arith.wast` -- see
    // `code/packages/rust/wasm-conformance/tests/fixtures/
    // fetch_testsuite.py`.
    SimdOpInfo { name: "f64x2.neg", sub_opcode: 0xED, kind: SimdOpKind::NegF64x2 },
    SimdOpInfo { name: "f64x2.sqrt", sub_opcode: 0xEF, kind: SimdOpKind::SqrtF64x2 },
    SimdOpInfo { name: "f64x2.add", sub_opcode: 0xF0, kind: SimdOpKind::AddF64x2 },
    SimdOpInfo { name: "f64x2.sub", sub_opcode: 0xF1, kind: SimdOpKind::SubF64x2 },
    SimdOpInfo { name: "f64x2.mul", sub_opcode: 0xF2, kind: SimdOpKind::MulF64x2 },
    SimdOpInfo { name: "f64x2.div", sub_opcode: 0xF3, kind: SimdOpKind::DivF64x2 },
    // SIMD widen PR32 (task #211-213): f64x2.eq (0x47), f64x2.ne (0x48),
    // f64x2.lt (0x49), f64x2.gt (0x4A), f64x2.le (0x4B), f64x2.ge
    // (0x4C) -- the f64x2 comparison family, a direct structural mirror
    // of PR30's f32x4 comparison family (0x41-0x46), just at f64x2's
    // 2-lane width. Each sub-opcode byte fetched live from
    // `BinarySIMD.md` and cross-checked against every existing
    // `SIMD_OPS` entry: `0x41`-`0x46` are the already-implemented
    // `f32x4` comparison family (PR30), `0x47`-`0x4C` sit immediately
    // past it with no overlap (confirmed distinct from the unrelated
    // `ATOMIC_OPS` table's own `0x47`-`0x4C` cmpxchg/xchg entries, which
    // live behind a completely different `0xFE` prefix, not `0xFD`), and
    // `v128.not` (`0x4D`, just above) confirms `0x47`-`0x4C` are
    // genuinely free. Like PR30, all six values are `< 0x80`
    // (single-byte LEB128, no continuation byte). This PR also vendors
    // `simd_f64x2_cmp.wast`, the single biggest directive-count win in
    // this campaign so far -- see
    // `code/packages/rust/wasm-conformance/tests/fixtures/
    // fetch_testsuite.py`.
    SimdOpInfo { name: "f64x2.eq", sub_opcode: 0x47, kind: SimdOpKind::EqF64x2 },
    SimdOpInfo { name: "f64x2.ne", sub_opcode: 0x48, kind: SimdOpKind::NeF64x2 },
    SimdOpInfo { name: "f64x2.lt", sub_opcode: 0x49, kind: SimdOpKind::LtF64x2 },
    SimdOpInfo { name: "f64x2.gt", sub_opcode: 0x4A, kind: SimdOpKind::GtF64x2 },
    SimdOpInfo { name: "f64x2.le", sub_opcode: 0x4B, kind: SimdOpKind::LeF64x2 },
    SimdOpInfo { name: "f64x2.ge", sub_opcode: 0x4C, kind: SimdOpKind::GeF64x2 },
    // SIMD widen PR33 (task #214-216): i8x16.add_sat_s/_u (0x6F/0x70),
    // i8x16.sub_sat_s/_u (0x72/0x73), i16x8.add_sat_s/_u (0x8F/0x90),
    // i16x8.sub_sat_s/_u (0x92/0x93) -- the saturating integer add/sub
    // family, BINARY same pop-order/lane-count shape as the
    // already-implemented `i8x16.add`/`.sub` (PR-early) and
    // `i16x8.add`/`.sub` (PR-early), except the result is CLAMPED to the
    // lane type's range instead of wrapped on overflow/underflow --
    // simpler than this crate's existing float `trunc_sat` handlers (no
    // NaN/infinity edge cases, just compute-in-a-wider-type-then-clamp-
    // then-cast on integer results), same clamp mechanic
    // `NarrowI16x8S/U`/`NarrowI32x4S/U` (PR27) already established for
    // this codebase, adapted from "clamp an already-computed wide
    // operand" to "clamp the result of a wide-intermediate add/sub".
    // Each sub-opcode byte fetched live from `BinarySIMD.md` and
    // cross-checked against every existing `SIMD_OPS` entry: `0x6F`/
    // `0x70` sit immediately past `i8x16.add`'s `0x6E` with no gap and
    // immediately before `i8x16.shl`'s `0x6B`..`i8x16.shr_u`'s `0x6D`
    // run and `i8x16.sub`'s `0x71` (both just outside this pair, no
    // overlap); `0x72`/`0x73` sit immediately past `i8x16.sub`'s `0x71`
    // with no gap; `0x8F`/`0x90` sit immediately past `i16x8.add`'s
    // `0x8E` with no gap; `0x92`/`0x93` sit immediately past
    // `i16x8.sub`'s `0x91` with no gap -- all eight confirmed free of
    // collision with every existing `SIMD_OPS` entry (PR27's `0x85`/
    // `0x86` `narrow_i32x4_s/_u` and PR26's `0x87`-`0x8A` `extend`
    // run sit well clear of the `0x8F`-`0x93` cluster). This PR also
    // vendors `simd_i8x16_sat_arith.wast` and `simd_i16x8_sat_arith.wast`
    // -- see `code/packages/rust/wasm-conformance/tests/fixtures/
    // fetch_testsuite.py`.
    SimdOpInfo { name: "i8x16.add_sat_s", sub_opcode: 0x6F, kind: SimdOpKind::AddSatI8x16S },
    SimdOpInfo { name: "i8x16.add_sat_u", sub_opcode: 0x70, kind: SimdOpKind::AddSatI8x16U },
    SimdOpInfo { name: "i8x16.sub_sat_s", sub_opcode: 0x72, kind: SimdOpKind::SubSatI8x16S },
    SimdOpInfo { name: "i8x16.sub_sat_u", sub_opcode: 0x73, kind: SimdOpKind::SubSatI8x16U },
    SimdOpInfo { name: "i16x8.add_sat_s", sub_opcode: 0x8F, kind: SimdOpKind::AddSatI16x8S },
    SimdOpInfo { name: "i16x8.add_sat_u", sub_opcode: 0x90, kind: SimdOpKind::AddSatI16x8U },
    SimdOpInfo { name: "i16x8.sub_sat_s", sub_opcode: 0x92, kind: SimdOpKind::SubSatI16x8S },
    SimdOpInfo { name: "i16x8.sub_sat_u", sub_opcode: 0x93, kind: SimdOpKind::SubSatI16x8U },
    // SIMD widen PR34 (task #217-219): f32x4.max (0xE9), f32x4.pmin
    // (0xEA), f32x4.pmax (0xEB) -- the last 3 opcodes of the f32x4
    // arithmetic family, sitting immediately past the already-implemented
    // `f32x4.min`'s `0xE8` (PR19) with no gap. Each sub-opcode byte
    // fetched live from `BinarySIMD.md` and cross-checked against
    // `f32x4.min`'s existing `0xE8` `SIMD_OPS` entry: `0xE9`/`0xEA`/`0xEB`
    // run contiguously past it, confirmed free of collision with every
    // existing `SIMD_OPS` entry (the next occupied slot above this run is
    // `i32x4.trunc_sat_f32x4_s` at `0xF8`, well clear). `f32x4.max`
    // mirrors `f32x4.min`'s WASM-spec `fmax` NaN-propagating,
    // signed-zero-aware semantics exactly (see `SimdOpKind::MaxF32x4`'s
    // own doc comment); `f32x4.pmin`/`f32x4.pmax` are a DIFFERENT,
    // deliberately SIMPLER "pseudo-min"/"pseudo-max" shape -- a plain
    // IEEE-754 `<`-based conditional select with no NaN canonicalization,
    // NOT the same code path as `min`/`max` (see `SimdOpKind::PminF32x4`/
    // `PmaxF32x4`'s own doc comments for the exact first-operand-wins-on-
    // NaN behavior this implies). This PR also vendors `simd_f32x4.wast`
    // and `simd_f32x4_pmin_pmax.wast` -- the best directive-per-opcode
    // ratio in this campaign so far -- see
    // `code/packages/rust/wasm-conformance/tests/fixtures/
    // fetch_testsuite.py`.
    SimdOpInfo { name: "f32x4.max", sub_opcode: 0xE9, kind: SimdOpKind::MaxF32x4 },
    SimdOpInfo { name: "f32x4.pmin", sub_opcode: 0xEA, kind: SimdOpKind::PminF32x4 },
    SimdOpInfo { name: "f32x4.pmax", sub_opcode: 0xEB, kind: SimdOpKind::PmaxF32x4 },
    // SIMD widen PR35 (task #220-222): f64x2.abs (0xEC), f64x2.min
    // (0xF4), f64x2.max (0xF5), f64x2.pmin (0xF6), f64x2.pmax (0xF7) --
    // closes the f64x2 arithmetic family, a direct structural mirror of
    // PR34's f32x4.max/pmin/pmax, plus `abs` (f32x4.abs already existed
    // since PR19; f64x2.abs did not exist yet, so it rides along here).
    // Each sub-opcode byte fetched live from `BinarySIMD.md` and
    // cross-checked against every existing `SIMD_OPS` entry: `0xEC` is
    // the slot immediately BEFORE the already-implemented `f64x2.neg`'s
    // `0xED` (PR31), previously called out as "still unimplemented" in
    // that PR's own comment above; `0xF4`/`0xF5` sit immediately past
    // `f64x2.div`'s `0xF3` (PR31) with no gap, likewise previously called
    // out as "still unimplemented"; `0xF6`/`0xF7` continue the run
    // immediately past `0xF5` with no gap, confirmed free of collision
    // with every existing `SIMD_OPS` entry (the next occupied slot above
    // this run is `i32x4.trunc_sat_f32x4_s` at `0xF8`, well clear, same
    // as PR34's own f32x4.max/pmin/pmax run). `f64x2.min`/`f64x2.max`
    // mirror `f32x4.min`/`f32x4.max`'s WASM-spec `fmin`/`fmax`
    // NaN-propagating, signed-zero-aware semantics exactly (see
    // `SimdOpKind::MinF64x2`/`MaxF64x2`'s own doc comments);
    // `f64x2.pmin`/`f64x2.pmax` are the same DIFFERENT, deliberately
    // SIMPLER "pseudo-min"/"pseudo-max" shape as their `f32x4` mirrors --
    // a plain IEEE-754 `<`-based conditional select with no NaN
    // canonicalization, NOT the same code path as `min`/`max` (see
    // `SimdOpKind::PminF64x2`/`PmaxF64x2`'s own doc comments for the
    // exact first-operand-wins-on-NaN behavior this implies). This PR
    // also vendors `simd_f64x2.wast` and `simd_f64x2_pmin_pmax.wast` --
    // see `code/packages/rust/wasm-conformance/tests/fixtures/
    // fetch_testsuite.py`.
    SimdOpInfo { name: "f64x2.abs", sub_opcode: 0xEC, kind: SimdOpKind::AbsF64x2 },
    SimdOpInfo { name: "f64x2.min", sub_opcode: 0xF4, kind: SimdOpKind::MinF64x2 },
    SimdOpInfo { name: "f64x2.max", sub_opcode: 0xF5, kind: SimdOpKind::MaxF64x2 },
    SimdOpInfo { name: "f64x2.pmin", sub_opcode: 0xF6, kind: SimdOpKind::PminF64x2 },
    SimdOpInfo { name: "f64x2.pmax", sub_opcode: 0xF7, kind: SimdOpKind::PmaxF64x2 },
];

/// Look up a SIMD opcode by its LEB128-decoded sub-opcode value (the
/// integer after the `0xFD` prefix byte).
///
/// # Example
///
/// ```
/// use wasm_opcodes::get_simd_op;
///
/// let info = get_simd_op(0x0C).unwrap();
/// assert_eq!(info.name, "v128.const");
/// ```
pub fn get_simd_op(sub_opcode: u32) -> Option<&'static SimdOpInfo> {
    SIMD_OPS.iter().find(|op| op.sub_opcode == sub_opcode)
}

/// Look up a SIMD opcode by its canonical text name, e.g. `"i32x4.add"`.
///
/// # Example
///
/// ```
/// use wasm_opcodes::get_simd_op_by_name;
///
/// let info = get_simd_op_by_name("i32x4.add").unwrap();
/// assert_eq!(info.sub_opcode, 0xAE);
/// ```
pub fn get_simd_op_by_name(name: &str) -> Option<&'static SimdOpInfo> {
    SIMD_OPS.iter().find(|op| op.name == name)
}

// ──────────────────────────────────────────────────────────────────────────────
// Public lookup API
// ──────────────────────────────────────────────────────────────────────────────

/// Look up an opcode by its byte value.
///
/// Returns `Some(&OpcodeInfo)` for any defined WASM 1.0 opcode byte, or
/// `None` for undefined bytes (gaps in the opcode space).
///
/// # Example
///
/// ```
/// use wasm_opcodes::get_opcode;
///
/// let info = get_opcode(0x6A).unwrap();
/// assert_eq!(info.name, "i32.add");
/// assert_eq!(info.stack_pop, 2);
/// assert_eq!(info.stack_push, 1);
/// ```
pub fn get_opcode(byte: u8) -> Option<&'static OpcodeInfo> {
    // Linear scan over 183 entries. At ~183 iterations maximum this is
    // negligible. A sorted array + binary search or a 256-slot lookup table
    // would give O(1) but adds complexity without measurable benefit here.
    OPCODES.iter().find(|op| op.opcode == byte)
}

/// Look up an opcode by its canonical text name.
///
/// Names are case-sensitive and use the standard WASM text format notation,
/// e.g. `"i32.add"`, `"call_indirect"`, `"f64.reinterpret_i64"`.
///
/// Returns `Some(&OpcodeInfo)` on a match, `None` if the name is unknown.
///
/// # Example
///
/// ```
/// use wasm_opcodes::get_opcode_by_name;
///
/// let info = get_opcode_by_name("i32.add").unwrap();
/// assert_eq!(info.opcode, 0x6A);
/// ```
pub fn get_opcode_by_name(name: &str) -> Option<&'static OpcodeInfo> {
    OPCODES.iter().find(|op| op.name == name)
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 1. Total opcode count covers all WASM 1.0 MVP instructions plus the
    //    sign-extension proposal's 5 (WASM03).
    //
    // The WASM 1.0 MVP spec defines 172 instructions across the byte range
    // 0x00–0xBF.  The gaps (e.g. 0x06–0x0A, 0x12–0x1F, 0x25–0x27) are
    // reserved/unassigned in the MVP — they are not valid opcodes.  The "~183"
    // figure sometimes cited counts proposals beyond MVP (SIMD, bulk-memory,
    // etc.) which use a two-byte 0xFC prefix encoding outside this table.
    #[test]
    fn test_total_count() {
        println!("Total opcodes: {}", OPCODES.len());
        assert!(
            OPCODES.len() >= 177,
            "Expected >= 177 opcodes (172 MVP + 5 sign-extension), got {}",
            OPCODES.len()
        );
    }

    // 2. get_opcode(0x6A) returns i32.add
    #[test]
    fn test_get_opcode_i32_add_by_byte() {
        let info = get_opcode(0x6A).expect("0x6A should be i32.add");
        assert_eq!(info.name, "i32.add");
    }

    // 3. get_opcode_by_name("i32.add") returns correct entry
    #[test]
    fn test_get_opcode_i32_add_by_name() {
        let info = get_opcode_by_name("i32.add").expect("i32.add should be found");
        assert_eq!(info.opcode, 0x6A);
    }

    // 4. i32.add: stack_pop=2, stack_push=1
    #[test]
    fn test_i32_add_stack_effects() {
        let info = get_opcode(0x6A).unwrap();
        assert_eq!(info.stack_pop, 2, "i32.add should pop 2");
        assert_eq!(info.stack_push, 1, "i32.add should push 1");
    }

    // 5. i32.const has immediates=["i32"]
    #[test]
    fn test_i32_const_immediates() {
        let info = get_opcode(0x41).expect("0x41 should be i32.const");
        assert_eq!(info.name, "i32.const");
        assert_eq!(info.immediates, &["i32"]);
    }

    // 6. i32.load has immediates=["memarg"]
    #[test]
    fn test_i32_load_immediates() {
        let info = get_opcode(0x28).expect("0x28 should be i32.load");
        assert_eq!(info.name, "i32.load");
        assert_eq!(info.immediates, &["memarg"]);
    }

    // 7. Unknown byte returns None
    #[test]
    fn test_unknown_byte_returns_none() {
        // 0x06..0x0A are unused in WASM 1.0
        assert!(get_opcode(0x06).is_none(), "0x06 is not a valid opcode");
        assert!(get_opcode(0xFF).is_none(), "0xFF is not a valid opcode");
    }

    // 8. Unknown name returns None
    #[test]
    fn test_unknown_name_returns_none() {
        assert!(get_opcode_by_name("i32.banana").is_none());
        assert!(get_opcode_by_name("").is_none());
    }

    // 9. All opcode bytes are unique
    #[test]
    fn test_all_bytes_unique() {
        let mut seen = std::collections::HashSet::new();
        for op in OPCODES {
            assert!(
                seen.insert(op.opcode),
                "Duplicate opcode byte: 0x{:02X} ({})",
                op.opcode,
                op.name
            );
        }
    }

    // 10. All names are unique
    #[test]
    fn test_all_names_unique() {
        let mut seen = std::collections::HashSet::new();
        for op in OPCODES {
            assert!(
                seen.insert(op.name),
                "Duplicate opcode name: {}",
                op.name
            );
        }
    }

    // 11. OPCODES count consistent with name lookup
    #[test]
    fn test_count_consistency() {
        let name_count = OPCODES
            .iter()
            .filter_map(|op| get_opcode_by_name(op.name))
            .count();
        assert_eq!(
            name_count,
            OPCODES.len(),
            "Every name in OPCODES should be findable by get_opcode_by_name"
        );
    }

    // Additional coverage: category spot checks
    #[test]
    fn test_categories() {
        assert_eq!(get_opcode(0x00).unwrap().category, "control");
        assert_eq!(get_opcode(0x1A).unwrap().category, "parametric");
        assert_eq!(get_opcode(0x20).unwrap().category, "variable");
        assert_eq!(get_opcode(0x28).unwrap().category, "memory");
        assert_eq!(get_opcode(0x41).unwrap().category, "numeric_i32");
        assert_eq!(get_opcode(0x42).unwrap().category, "numeric_i64");
        assert_eq!(get_opcode(0x43).unwrap().category, "numeric_f32");
        assert_eq!(get_opcode(0x44).unwrap().category, "numeric_f64");
        assert_eq!(get_opcode(0xA7).unwrap().category, "conversion");
    }

    // Additional: call_indirect has two immediates
    #[test]
    fn test_call_indirect_immediates() {
        let info = get_opcode(0x11).unwrap();
        assert_eq!(info.name, "call_indirect");
        assert_eq!(info.immediates, &["typeidx", "tableidx"]);
        assert_eq!(info.stack_pop, 1);
    }

    // Additional: select has stack_pop=3, stack_push=1
    #[test]
    fn test_select_stack() {
        let info = get_opcode_by_name("select").unwrap();
        assert_eq!(info.stack_pop, 3);
        assert_eq!(info.stack_push, 1);
    }

    // Additional: memory.grow pops 1, pushes 1
    #[test]
    fn test_memory_grow_stack() {
        let info = get_opcode(0x40).unwrap();
        assert_eq!(info.name, "memory.grow");
        assert_eq!(info.stack_pop, 1);
        assert_eq!(info.stack_push, 1);
    }

    // Additional: conversion instructions all have pop=1, push=1, no immediates
    #[test]
    fn test_conversions_stack_effects() {
        let conversions = OPCODES
            .iter()
            .filter(|op| op.category == "conversion");
        for op in conversions {
            assert_eq!(op.stack_pop, 1, "{} should pop 1", op.name);
            assert_eq!(op.stack_push, 1, "{} should push 1", op.name);
            assert!(op.immediates.is_empty(), "{} should have no immediates", op.name);
        }
    }

    // Additional: f64.reinterpret_i64 exists at 0xBF
    #[test]
    fn test_f64_reinterpret_i64() {
        let info = get_opcode(0xBF).expect("0xBF should be f64.reinterpret_i64");
        assert_eq!(info.name, "f64.reinterpret_i64");
    }

    // Additional (WASM03): the 5 sign-extension opcodes round-trip by both
    // byte and name, at their real spec-assigned bytes (0xC0-0xC4).
    #[test]
    fn test_sign_extension_opcodes() {
        let expected = [
            (0xC0u8, "i32.extend8_s"),
            (0xC1, "i32.extend16_s"),
            (0xC2, "i64.extend8_s"),
            (0xC3, "i64.extend16_s"),
            (0xC4, "i64.extend32_s"),
        ];
        for (byte, name) in expected {
            let by_byte = get_opcode(byte).unwrap_or_else(|| panic!("{byte:#04x} should be {name}"));
            assert_eq!(by_byte.name, name);
            let by_name = get_opcode_by_name(name).unwrap_or_else(|| panic!("{name} should be found"));
            assert_eq!(by_name.opcode, byte);
        }
    }

    // Additional (WASM17): the reference-types proposal's new opcodes
    // round-trip by both byte and name, with the expected stack shape and
    // immediates. `table.get`/`table.set` sit in the previously-reserved
    // 0x25/0x26 MVP gap; `ref.func` is at 0xD2. `ref.null` (0xD0) and
    // `ref.is_null` (0xD1) are deliberately absent from this table (see the
    // comment above `ref.func`'s entry) and so must NOT be found here.
    #[test]
    fn test_reference_types_opcodes() {
        let table_get = get_opcode(0x25).expect("0x25 should be table.get");
        assert_eq!(table_get.name, "table.get");
        assert_eq!(table_get.immediates, &["tableidx"]);
        assert_eq!(table_get.stack_pop, 1);
        assert_eq!(table_get.stack_push, 1);

        let table_set = get_opcode(0x26).expect("0x26 should be table.set");
        assert_eq!(table_set.name, "table.set");
        assert_eq!(table_set.immediates, &["tableidx"]);
        assert_eq!(table_set.stack_pop, 2);
        assert_eq!(table_set.stack_push, 0);

        let ref_func = get_opcode(0xD2).expect("0xD2 should be ref.func");
        assert_eq!(ref_func.name, "ref.func");
        assert_eq!(ref_func.immediates, &["funcidx"]);
        assert_eq!(ref_func.stack_pop, 0);
        assert_eq!(ref_func.stack_push, 1);

        assert_eq!(get_opcode_by_name("table.get").map(|o| o.opcode), Some(0x25));
        assert_eq!(get_opcode_by_name("table.set").map(|o| o.opcode), Some(0x26));
        assert_eq!(get_opcode_by_name("ref.func").map(|o| o.opcode), Some(0xD2));

        assert!(get_opcode(0xD0).is_none(), "ref.null is intentionally not in this table");
        assert!(get_opcode(0xD1).is_none(), "ref.is_null is intentionally not in this table");
    }

    // ── WASM16: tail calls ────────────────────────────────────────────────

    #[test]
    fn test_tail_call_opcodes() {
        let return_call = get_opcode(0x12).expect("0x12 should be return_call");
        assert_eq!(return_call.name, "return_call");
        assert_eq!(return_call.immediates, &["funcidx"]);

        let return_call_indirect = get_opcode(0x13).expect("0x13 should be return_call_indirect");
        assert_eq!(return_call_indirect.name, "return_call_indirect");
        assert_eq!(return_call_indirect.immediates, &["typeidx", "tableidx"]);

        assert_eq!(get_opcode_by_name("return_call").map(|o| o.opcode), Some(0x12));
        assert_eq!(get_opcode_by_name("return_call_indirect").map(|o| o.opcode), Some(0x13));
    }

    // ── WASM18: atomic memory operations (0xFE prefix) ───────────────────────

    #[test]
    fn atomic_ops_table_has_the_expected_count_and_no_duplicates() {
        // notify + wait32 + wait64 + fence + 7 loads + 7 stores + (6 RMW
        // op kinds + cmpxchg = 7 op kinds) * 7 width variants each =
        // 3 + 1 + 7 + 7 + 49 = 67.
        assert_eq!(ATOMIC_OPS.len(), 3 + 1 + 7 + 7 + 7 * 7);

        let mut seen_bytes = std::collections::HashSet::new();
        let mut seen_names = std::collections::HashSet::new();
        for op in ATOMIC_OPS {
            assert!(seen_bytes.insert(op.sub_opcode), "duplicate sub-opcode byte {:#04x} ({})", op.sub_opcode, op.name);
            assert!(seen_names.insert(op.name), "duplicate name {}", op.name);
        }
    }

    #[test]
    fn atomic_ops_byte_range_is_contiguous_and_matches_the_spec() {
        // notify/wait32/wait64/fence at 0x00-0x03, loads+stores
        // 0x10-0x1D, RMW+cmpxchg 0x1E-0x4E -- every byte in 0x00..=0x4E
        // is now a defined atomic op (0x04-0x0F is the only real gap).
        for byte in 0x00..=0x03 {
            assert!(get_atomic_op(byte).is_some(), "sub-opcode {byte:#04x} should be a defined sync op");
        }
        for byte in 0x10..=0x4E {
            assert!(get_atomic_op(byte).is_some(), "sub-opcode {byte:#04x} should be a defined atomic op");
        }
        for byte in 0x04..=0x0F {
            assert!(get_atomic_op(byte).is_none(), "sub-opcode {byte:#04x} is a real gap in the encoding, not assigned");
        }
    }

    #[test]
    fn atomic_ops_spot_check_loads_stores_rmw_cmpxchg() {
        let load = get_atomic_op(0x10).expect("0x10");
        assert_eq!(load.name, "i32.atomic.load");
        assert_eq!(load.kind, AtomicOpKind::Load);
        assert_eq!(load.value_type, Some(wasm_types::ValueType::I32));
        assert_eq!(load.natural_align, 4);

        let store8 = get_atomic_op(0x1B).expect("0x1B");
        assert_eq!(store8.name, "i64.atomic.store8");
        assert_eq!(store8.kind, AtomicOpKind::Store);
        assert_eq!(store8.value_type, Some(wasm_types::ValueType::I64));
        assert_eq!(store8.natural_align, 1);

        // First RMW op (add, i32) and last (cmpxchg, i64.rmw32).
        let rmw_add = get_atomic_op(0x1E).expect("0x1E");
        assert_eq!(rmw_add.name, "i32.atomic.rmw.add");
        assert_eq!(rmw_add.kind, AtomicOpKind::Rmw);

        let cmpxchg = get_atomic_op(0x4E).expect("0x4E");
        assert_eq!(cmpxchg.name, "i64.atomic.rmw32.cmpxchg_u");
        assert_eq!(cmpxchg.kind, AtomicOpKind::Cmpxchg);
        assert_eq!(cmpxchg.value_type, Some(wasm_types::ValueType::I64));
        assert_eq!(cmpxchg.natural_align, 4);

        let fence = get_atomic_op(0x03).expect("0x03");
        assert_eq!(fence.kind, AtomicOpKind::Fence);
        assert_eq!(fence.value_type, None);

        assert_eq!(get_atomic_op_by_name("i32.atomic.load").map(|o| o.sub_opcode), Some(0x10));
        assert_eq!(get_atomic_op_by_name("i64.atomic.rmw32.cmpxchg_u").map(|o| o.sub_opcode), Some(0x4E));
    }

    #[test]
    fn atomic_ops_notify_and_wait_are_present_with_correct_shapes() {
        // Implementation-time correction of the (already-merged) W09
        // spec's own claim that these are "deliberately absent -- see
        // AtomicOpKind::Notify/Wait's own doc comments for why the real,
        // pinned-commit testsuite proved otherwise.
        let notify = get_atomic_op(0x00).expect("0x00");
        assert_eq!(notify.name, "memory.atomic.notify");
        assert_eq!(notify.kind, AtomicOpKind::Notify);
        assert_eq!(notify.value_type, None);

        let wait32 = get_atomic_op(0x01).expect("0x01");
        assert_eq!(wait32.name, "memory.atomic.wait32");
        assert_eq!(wait32.kind, AtomicOpKind::Wait);
        assert_eq!(wait32.value_type, Some(wasm_types::ValueType::I32));

        let wait64 = get_atomic_op(0x02).expect("0x02");
        assert_eq!(wait64.name, "memory.atomic.wait64");
        assert_eq!(wait64.kind, AtomicOpKind::Wait);
        assert_eq!(wait64.value_type, Some(wasm_types::ValueType::I64));

        assert_eq!(get_atomic_op_by_name("memory.atomic.notify").map(|o| o.sub_opcode), Some(0x00));
    }

    // ── SIMD (0xFD prefix, v128 first slice) ─────────────────────────────────

    #[test]
    fn simd_ops_table_has_the_expected_207_entries_and_no_duplicates() {
        assert_eq!(SIMD_OPS.len(), 207);

        let mut seen_sub_opcodes = std::collections::HashSet::new();
        let mut seen_names = std::collections::HashSet::new();
        for op in SIMD_OPS {
            assert!(seen_sub_opcodes.insert(op.sub_opcode), "duplicate sub-opcode {:#04x} ({})", op.sub_opcode, op.name);
            assert!(seen_names.insert(op.name), "duplicate name {}", op.name);
        }
    }

    #[test]
    fn simd_ops_have_the_real_verified_sub_opcode_values() {
        // Verified against the SIMD proposal's own BinarySIMD.md AND the
        // W3C core spec's "Vector Instructions" section independently --
        // see code/specs/W13-wasm-simd-v128-first-slice.md.
        let v128_const = get_simd_op(0x0C).expect("0x0C should be v128.const");
        assert_eq!(v128_const.name, "v128.const");
        assert_eq!(v128_const.kind, SimdOpKind::Const);

        let splat = get_simd_op(0x11).expect("0x11 should be i32x4.splat");
        assert_eq!(splat.name, "i32x4.splat");
        assert_eq!(splat.kind, SimdOpKind::Splat);

        let eq = get_simd_op(0x37).expect("0x37 should be i32x4.eq");
        assert_eq!(eq.name, "i32x4.eq");
        assert_eq!(eq.kind, SimdOpKind::Eq);

        let extract_lane = get_simd_op(0x1B).expect("0x1B should be i32x4.extract_lane");
        assert_eq!(extract_lane.name, "i32x4.extract_lane");
        assert_eq!(extract_lane.kind, SimdOpKind::ExtractLane);

        // i32x4.add's real sub-opcode is 174 (0xAE) -- deliberately >= 128,
        // the one entry in this first slice that genuinely exercises the
        // multi-byte LEB128 decode path, not just its single-byte-safe
        // happy path (v128.const/splat/eq are all < 128).
        let add = get_simd_op(174).expect("174 (0xAE) should be i32x4.add");
        assert_eq!(add.name, "i32x4.add");
        assert_eq!(add.kind, SimdOpKind::Add);
        assert_eq!(add.sub_opcode, 0xAE);

        assert_eq!(get_simd_op_by_name("i32x4.add").map(|o| o.sub_opcode), Some(174));
    }

    #[test]
    fn simd_op_unknown_sub_opcode_returns_none() {
        assert!(get_simd_op(0xFFFF).is_none());
        assert!(get_simd_op_by_name("i8x16.shuffle").is_none(), "not yet implemented in this first slice");
    }

    #[test]
    fn simd_i32x4_arith_and_cmp_widening_has_the_real_verified_sub_opcode_values() {
        // Fetched live from the SIMD proposal's BinarySIMD.md, cross-
        // checked against the already-implemented i32x4.eq (0x37) and
        // i32x4.add (0xAE) entries (both matched exactly) -- see this
        // package's own CHANGELOG entry for the widening PR.
        for (name, sub_opcode, kind) in [
            ("i32x4.ne", 0x38, SimdOpKind::Ne),
            ("i32x4.lt_s", 0x39, SimdOpKind::LtS),
            ("i32x4.lt_u", 0x3A, SimdOpKind::LtU),
            ("i32x4.gt_s", 0x3B, SimdOpKind::GtS),
            ("i32x4.gt_u", 0x3C, SimdOpKind::GtU),
            ("i32x4.le_s", 0x3D, SimdOpKind::LeS),
            ("i32x4.le_u", 0x3E, SimdOpKind::LeU),
            ("i32x4.ge_s", 0x3F, SimdOpKind::GeS),
            ("i32x4.ge_u", 0x40, SimdOpKind::GeU),
            ("i32x4.neg", 0xA1, SimdOpKind::Neg),
            ("i32x4.sub", 0xB1, SimdOpKind::Sub),
            ("i32x4.mul", 0xB5, SimdOpKind::Mul),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i32x4_arith2_widening_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md, cross-checked against the
        // already-implemented i32x4.eq (0x37)/i32x4.add (0xAE) entries
        // (both matched exactly) -- same discipline as the arith/cmp
        // widening above. See this package's own CHANGELOG entry.
        for (name, sub_opcode, kind) in [
            ("i32x4.abs", 0xA0, SimdOpKind::Abs),
            ("i32x4.min_s", 0xB6, SimdOpKind::MinS),
            ("i32x4.min_u", 0xB7, SimdOpKind::MinU),
            ("i32x4.max_s", 0xB8, SimdOpKind::MaxS),
            ("i32x4.max_u", 0xB9, SimdOpKind::MaxU),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i32x4_from_i16x8_widening_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md, cross-checked against the
        // already-implemented i32x4.eq (0x37)/i32x4.add (0xAE) entries
        // (both matched exactly) -- same discipline as every widening
        // above. The first opcodes in this table whose INPUT lane width
        // (i16x8) differs from their OUTPUT lane width (i32x4).
        for (name, sub_opcode, kind) in [
            ("i32x4.extadd_pairwise_i16x8_s", 0x7E, SimdOpKind::ExtaddPairwiseI16x8S),
            ("i32x4.extadd_pairwise_i16x8_u", 0x7F, SimdOpKind::ExtaddPairwiseI16x8U),
            ("i32x4.dot_i16x8_s", 0xBA, SimdOpKind::DotI16x8S),
            ("i32x4.extmul_low_i16x8_s", 0xBC, SimdOpKind::ExtmulLowI16x8S),
            ("i32x4.extmul_high_i16x8_s", 0xBD, SimdOpKind::ExtmulHighI16x8S),
            ("i32x4.extmul_low_i16x8_u", 0xBE, SimdOpKind::ExtmulLowI16x8U),
            ("i32x4.extmul_high_i16x8_u", 0xBF, SimdOpKind::ExtmulHighI16x8U),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i8x16_first_slice_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md, cross-checked against the
        // already-implemented i32x4.add (0xAE)/i32x4.abs (0xA0) entries
        // (both matched exactly) -- same discipline as every prior
        // addition. This is a brand-new lane width's first slice (like
        // i32x4's own original 5-opcode slice), not a widening of an
        // existing one -- no i8x16.mul exists in the spec (8-bit lanes
        // are too narrow for a useful lane-wise multiply).
        for (name, sub_opcode, kind) in [
            ("i8x16.neg", 0x61, SimdOpKind::NegI8x16),
            ("i8x16.add", 0x6E, SimdOpKind::AddI8x16),
            ("i8x16.sub", 0x71, SimdOpKind::SubI8x16),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
        assert!(get_simd_op_by_name("i8x16.mul").is_none(), "WASM SIMD defines no i8x16.mul");
    }

    #[test]
    fn simd_i16x8_first_slice_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md, cross-checked against the
        // already-implemented i32x4.add (0xAE)/i8x16.add (0x6E) entries
        // (both matched exactly) -- same discipline as every prior
        // addition. The first opcodes in this table where i16x8 is a
        // PRIMARY lane width (produces i16x8 results), not just read as
        // a widening-op's input (ExtaddPairwiseI16x8S/DotI16x8S/etc.).
        // Unlike i8x16, WASM SIMD DOES define i16x8.mul.
        for (name, sub_opcode, kind) in [
            ("i16x8.neg", 0x81, SimdOpKind::NegI16x8),
            ("i16x8.add", 0x8E, SimdOpKind::AddI16x8),
            ("i16x8.sub", 0x91, SimdOpKind::SubI16x8),
            ("i16x8.mul", 0x95, SimdOpKind::MulI16x8),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i16x8_cmp_family_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md, cross-checked against the
        // already-implemented i16x8.add (0x8E)/i32x4.eq (0x37) entries
        // (both matched exactly) -- same discipline as every prior
        // addition. Closes the gap left when i16x8.add/sub/mul/neg landed
        // without a comparison family (unlike i32x4, which got arith+cmp
        // together) -- same boolean-mask convention and signed/unsigned
        // split as i32x4's own comparison family, just at i16x8's width.
        for (name, sub_opcode, kind) in [
            ("i16x8.eq", 0x2D, SimdOpKind::EqI16x8),
            ("i16x8.ne", 0x2E, SimdOpKind::NeI16x8),
            ("i16x8.lt_s", 0x2F, SimdOpKind::LtSI16x8),
            ("i16x8.lt_u", 0x30, SimdOpKind::LtUI16x8),
            ("i16x8.gt_s", 0x31, SimdOpKind::GtSI16x8),
            ("i16x8.gt_u", 0x32, SimdOpKind::GtUI16x8),
            ("i16x8.le_s", 0x33, SimdOpKind::LeSI16x8),
            ("i16x8.le_u", 0x34, SimdOpKind::LeUI16x8),
            ("i16x8.ge_s", 0x35, SimdOpKind::GeSI16x8),
            ("i16x8.ge_u", 0x36, SimdOpKind::GeUI16x8),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i8x16_cmp_family_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md, cross-checked against the
        // already-implemented i8x16.add (0x6E)/i16x8.eq (0x2D) entries
        // (both matched exactly) -- same discipline as every prior
        // addition. Closes the same gap for i8x16 that i16x8.eq/ne/etc.
        // (task #133-136) just closed for i16x8: i8x16 had arith
        // (add/sub/neg) but no comparison family until now.
        for (name, sub_opcode, kind) in [
            ("i8x16.eq", 0x23, SimdOpKind::EqI8x16),
            ("i8x16.ne", 0x24, SimdOpKind::NeI8x16),
            ("i8x16.lt_s", 0x25, SimdOpKind::LtSI8x16),
            ("i8x16.lt_u", 0x26, SimdOpKind::LtUI8x16),
            ("i8x16.gt_s", 0x27, SimdOpKind::GtSI8x16),
            ("i8x16.gt_u", 0x28, SimdOpKind::GtUI8x16),
            ("i8x16.le_s", 0x29, SimdOpKind::LeSI8x16),
            ("i8x16.le_u", 0x2A, SimdOpKind::LeUI8x16),
            ("i8x16.ge_s", 0x2B, SimdOpKind::GeSI8x16),
            ("i8x16.ge_u", 0x2C, SimdOpKind::GeUI8x16),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i8x16_arith2_family_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md, cross-checked against the
        // already-implemented i8x16.add (0x6E)/i8x16.neg (0x61)/
        // i8x16.sub (0x71) entries (all three matched exactly) -- same
        // discipline as every prior addition. Mirrors i32x4's own
        // abs/min_s/min_u/max_s/max_u "arith2" widening (task #118-120),
        // plus two op SHAPES with no i32x4/i16x8 precedent: popcnt and
        // avgr_u (WASM SIMD defines popcnt/avgr_u only for i8x16).
        for (name, sub_opcode, kind) in [
            ("i8x16.abs", 0x60, SimdOpKind::AbsI8x16),
            ("i8x16.popcnt", 0x62, SimdOpKind::PopcntI8x16),
            ("i8x16.min_s", 0x76, SimdOpKind::MinSI8x16),
            ("i8x16.min_u", 0x77, SimdOpKind::MinUI8x16),
            ("i8x16.max_s", 0x78, SimdOpKind::MaxSI8x16),
            ("i8x16.max_u", 0x79, SimdOpKind::MaxUI8x16),
            ("i8x16.avgr_u", 0x7B, SimdOpKind::AvgrUI8x16),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i16x8_arith2_family_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md, cross-checked against the
        // already-implemented i16x8.neg (0x81)/add (0x8E)/sub (0x91)/
        // mul (0x95) entries (all four matched exactly) -- same
        // discipline as every prior addition. Closes the same "arith2"
        // gap for i16x8 that PR8 just closed for i8x16 (no
        // i16x8.popcnt -- WASM SIMD only defines popcnt for i8x16).
        for (name, sub_opcode, kind) in [
            ("i16x8.abs", 0x80, SimdOpKind::AbsI16x8),
            ("i16x8.min_s", 0x96, SimdOpKind::MinSI16x8),
            ("i16x8.min_u", 0x97, SimdOpKind::MinUI16x8),
            ("i16x8.max_s", 0x98, SimdOpKind::MaxSI16x8),
            ("i16x8.max_u", 0x99, SimdOpKind::MaxUI16x8),
            ("i16x8.avgr_u", 0x9B, SimdOpKind::AvgrUI16x8),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i16x8_from_i8x16_widening_family_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md, cross-checked against the
        // already-implemented i8x16.add (0x6E)/i16x8.mul (0x95)/
        // i16x8.avgr_u (0x9B)/i32x4.dot_i16x8_s (0xBA)/i8x16.popcnt
        // (0x62)/i32x4.extadd_pairwise_i16x8_s (0x7E) entries (all six
        // matched exactly) -- same discipline as every prior addition.
        // Mirrors the already-implemented i32x4-from-i16x8 widening
        // family (task #121-124) one lane width down, closing the last
        // remaining gap between i16x8 and i8x16's coverage. No
        // i16x8.dot_i8x16_s -- WASM SIMD does not define a dot-product
        // for this lane-width pair.
        for (name, sub_opcode, kind) in [
            ("i16x8.extadd_pairwise_i8x16_s", 0x7C, SimdOpKind::ExtaddPairwiseI8x16S),
            ("i16x8.extadd_pairwise_i8x16_u", 0x7D, SimdOpKind::ExtaddPairwiseI8x16U),
            ("i16x8.extmul_low_i8x16_s", 0x9C, SimdOpKind::ExtmulLowI8x16S),
            ("i16x8.extmul_high_i8x16_s", 0x9D, SimdOpKind::ExtmulHighI8x16S),
            ("i16x8.extmul_low_i8x16_u", 0x9E, SimdOpKind::ExtmulLowI8x16U),
            ("i16x8.extmul_high_i8x16_u", 0x9F, SimdOpKind::ExtmulHighI8x16U),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_bitwise_family_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md, cross-checked against the
        // already-implemented i8x16.add (0x6E)/i32x4.add (0xAE) entries
        // (both matched exactly) -- same discipline as every prior
        // addition. Closes the gap between the narrow per-lane-width
        // arithmetic families done in PR1-10 and the far more
        // universally-used masking/blending idioms every real SIMD
        // program relies on. `bitselect` is the first TERNARY SIMD op
        // in this table.
        for (name, sub_opcode, kind) in [
            ("v128.not", 0x4D, SimdOpKind::Not),
            ("v128.and", 0x4E, SimdOpKind::And),
            ("v128.andnot", 0x4F, SimdOpKind::AndNot),
            ("v128.or", 0x50, SimdOpKind::Or),
            ("v128.xor", 0x51, SimdOpKind::Xor),
            ("v128.bitselect", 0x52, SimdOpKind::Bitselect),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_boolean_reduction_and_bitmask_family_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md. `v128.any_true` (0x53)
        // immediately follows the already-implemented `v128.bitselect`
        // (0x52); each lane width's `all_true`/`bitmask` pair sits at
        // base+3/base+4 relative to that lane width's already-implemented
        // `abs`/`neg` pair (i8x16.popcnt at 0x62 -> all_true 0x63 ->
        // bitmask 0x64; i16x8.abs 0x80/neg 0x81 -> all_true 0x83 ->
        // bitmask 0x84; i32x4.abs 0xA0/neg 0xA1 -> all_true 0xA3 ->
        // bitmask 0xA4; i64x2 follows the same 0xC0/0xC1/0xC3/0xC4
        // pattern, the first i64x2 opcodes in this table) -- same
        // discipline as every prior addition. This is the first
        // v128-in/i32-out reduction shape besides `extract_lane`, and the
        // first opcodes to read the operand as 8-byte (`i64`) lanes.
        for (name, sub_opcode, kind) in [
            ("v128.any_true", 0x53, SimdOpKind::AnyTrue),
            ("i8x16.all_true", 0x63, SimdOpKind::AllTrueI8x16),
            ("i8x16.bitmask", 0x64, SimdOpKind::BitmaskI8x16),
            ("i16x8.all_true", 0x83, SimdOpKind::AllTrueI16x8),
            ("i16x8.bitmask", 0x84, SimdOpKind::BitmaskI16x8),
            ("i32x4.all_true", 0xA3, SimdOpKind::AllTrueI32x4),
            ("i32x4.bitmask", 0xA4, SimdOpKind::BitmaskI32x4),
            ("i64x2.all_true", 0xC3, SimdOpKind::AllTrueI64x2),
            ("i64x2.bitmask", 0xC4, SimdOpKind::BitmaskI64x2),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i64x2_arith_and_cmp_family_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md (twice, identical both times).
        // `i64x2.abs` (0xC0)/`neg` (0xC1) fill the gap left by PR12's
        // `all_true` (0xC3)/`bitmask` (0xC4), matching the identical
        // abs/neg/[gap]/all_true/bitmask cluster layout already confirmed
        // for i8x16 (0x60/0x61/../0x63/0x64), i16x8 (0x80/0x81/../0x83/
        // 0x84), and i32x4 (0xA0/0xA1/../0xA3/0xA4). `add`/`sub`/`mul`
        // (0xCE/0xD1/0xD5) and the `eq`..`ge_s` comparison family form
        // one contiguous 0xD5-0xDB run, matching the contiguous cmp
        // blocks of every other lane width. No `lt_u`/`gt_u`/`le_u`/
        // `ge_u` -- the SIMD proposal never defines unsigned i64x2
        // comparisons, unlike every narrower lane width. This is i64x2's
        // first REAL ARITHMETIC family (PR12 only added the all_true/
        // bitmask reduction ops).
        for (name, sub_opcode, kind) in [
            ("i64x2.abs", 0xC0, SimdOpKind::AbsI64x2),
            ("i64x2.neg", 0xC1, SimdOpKind::NegI64x2),
            ("i64x2.add", 0xCE, SimdOpKind::AddI64x2),
            ("i64x2.sub", 0xD1, SimdOpKind::SubI64x2),
            ("i64x2.mul", 0xD5, SimdOpKind::MulI64x2),
            ("i64x2.eq", 0xD6, SimdOpKind::EqI64x2),
            ("i64x2.ne", 0xD7, SimdOpKind::NeI64x2),
            ("i64x2.lt_s", 0xD8, SimdOpKind::LtSI64x2),
            ("i64x2.gt_s", 0xD9, SimdOpKind::GtSI64x2),
            ("i64x2.le_s", 0xDA, SimdOpKind::LeSI64x2),
            ("i64x2.ge_s", 0xDB, SimdOpKind::GeSI64x2),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_shift_family_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md. Each width's shl/shr_s/shr_u
        // triple sits immediately BEFORE that width's already-implemented
        // `add` sub-opcode (e.g. i8x16.shl=0x6B, shr_s=0x6C, shr_u=0x6D,
        // then i8x16.add=0x6E; i16x8.shl=0x8B/../shr_u=0x8D, then
        // i16x8.add=0x8E; i32x4.shl=0xAB/../shr_u=0xAD, then
        // i32x4.add=0xAE; i64x2.shl=0xCB/../shr_u=0xCD, then
        // i64x2.add=0xCE) -- the same regular numbering scheme already
        // confirmed for every other family in this table. This is the
        // FIRST mixed-type binary SIMD op family: pops a scalar `i32`
        // shift amount (masked modulo the lane's bit width per the SIMD
        // spec) then a `v128`, pushes one `v128` -- every op before this
        // one pops only `v128`s (or, for `splat`/`extract_lane`, exactly
        // one `i32`/`v128`, never a MIX of both types in one op).
        for (name, sub_opcode, kind) in [
            ("i8x16.shl", 0x6B, SimdOpKind::ShlI8x16),
            ("i8x16.shr_s", 0x6C, SimdOpKind::ShrSI8x16),
            ("i8x16.shr_u", 0x6D, SimdOpKind::ShrUI8x16),
            ("i16x8.shl", 0x8B, SimdOpKind::ShlI16x8),
            ("i16x8.shr_s", 0x8C, SimdOpKind::ShrSI16x8),
            ("i16x8.shr_u", 0x8D, SimdOpKind::ShrUI16x8),
            ("i32x4.shl", 0xAB, SimdOpKind::ShlI32x4),
            ("i32x4.shr_s", 0xAC, SimdOpKind::ShrSI32x4),
            ("i32x4.shr_u", 0xAD, SimdOpKind::ShrUI32x4),
            ("i64x2.shl", 0xCB, SimdOpKind::ShlI64x2),
            ("i64x2.shr_s", 0xCC, SimdOpKind::ShrSI64x2),
            ("i64x2.shr_u", 0xCD, SimdOpKind::ShrUI64x2),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_load_and_store_have_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md. The FIRST SIMD load/store
        // opcodes in this table -- `v128.load` (0x00) is the very first
        // 0xFD sub-opcode of all (immediately before `v128.load8x8_s`
        // (0x01), the next extended-load variant this repo doesn't
        // implement yet); `v128.store` (0x0B) sits right before
        // `v128.const` (0x0C, already implemented) in the same
        // contiguous load/store/const cluster. Both carry a `memarg`
        // immediate, like every scalar `iNN.load`/`iNN.store` -- not the
        // 16-byte raw literal `v128.const` uses, and not the
        // no-immediate shape most other SIMD ops in this table use.
        for (name, sub_opcode, kind) in [
            ("v128.load", 0x00, SimdOpKind::Load),
            ("v128.store", 0x0B, SimdOpKind::Store),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_splat_family_has_the_real_verified_sub_opcode_values() {
        // Fetched live from BinarySIMD.md and cross-checked against the
        // already-implemented `i32x4.splat` (0x11) entry, which sits
        // exactly in the middle of this contiguous run: `i8x16.splat`
        // (0x0F), `i16x8.splat` (0x10), `i32x4.splat` (0x11, already
        // implemented), `i64x2.splat` (0x12). All three new entries
        // reuse the exact "pop one scalar, push one v128" shape
        // `i32x4.splat` already established -- `i64x2.splat` is the
        // first splat whose popped operand type is `i64` rather than
        // `i32`.
        for (name, sub_opcode, kind) in [
            ("i8x16.splat", 0x0F, SimdOpKind::SplatI8x16),
            ("i16x8.splat", 0x10, SimdOpKind::SplatI16x8),
            ("i64x2.splat", 0x12, SimdOpKind::SplatI64x2),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_float_splat_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR17: `f32x4.splat` (0x13) and `f64x2.splat`
        // (0x14) -- the immediate continuation of PR16's splat family
        // run, and the FIRST floating-point-typed SIMD ops in this
        // table. Fetched live from BinarySIMD.md and cross-checked
        // against the already-implemented `i64x2.splat` (0x12) entry
        // (both matched exactly, confirming the whole 0x0F-0x14 splat
        // run is contiguous and self-consistent). Splat itself is a
        // pure bit-pattern broadcast -- no rounding, no NaN
        // canonicalization, no comparison semantics -- so it needs no
        // new operand shape beyond popping `F32`/`F64` instead of
        // `I32`/`I64`.
        for (name, sub_opcode, kind) in [("f32x4.splat", 0x13, SimdOpKind::SplatF32x4), ("f64x2.splat", 0x14, SimdOpKind::SplatF64x2)] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i8x16_swizzle_and_lane_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR18: `i8x16.swizzle` (0x0E), `i8x16.extract_lane_s`
        // (0x15), `i8x16.extract_lane_u` (0x16), `i8x16.replace_lane`
        // (0x17) -- fetched live from BinarySIMD.md and cross-checked
        // against the already-implemented `i32x4.extract_lane` (0x1B)
        // and `i8x16.eq` (0x23) entries, which sit exactly one past this
        // run's own end (both matched exactly, confirming the whole
        // 0x0C-0x23 const/splat/extract_lane/eq encoding run is
        // contiguous and self-consistent).
        for (name, sub_opcode, kind) in [
            ("i8x16.swizzle", 0x0E, SimdOpKind::Swizzle),
            ("i8x16.extract_lane_s", 0x15, SimdOpKind::ExtractLaneI8x16S),
            ("i8x16.extract_lane_u", 0x16, SimdOpKind::ExtractLaneI8x16U),
            ("i8x16.replace_lane", 0x17, SimdOpKind::ReplaceLaneI8x16),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_extract_replace_lane_family_pr37_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR37: the remaining extract_lane/replace_lane family
        // members across i16x8/i32x4/i64x2/f32x4/f64x2 -- fetched live
        // from BinarySIMD.md and cross-checked against the already-
        // implemented i8x16.extract_lane_s (0x15)/extract_lane_u (0x16)/
        // replace_lane (0x17) and i32x4.extract_lane (0x1B) entries
        // (all four matched exactly, confirming the whole 0x15-0x22
        // lane-op run is contiguous and self-consistent).
        for (name, sub_opcode, kind) in [
            ("i16x8.extract_lane_s", 0x18, SimdOpKind::ExtractLaneI16x8S),
            ("i16x8.extract_lane_u", 0x19, SimdOpKind::ExtractLaneI16x8U),
            ("i16x8.replace_lane", 0x1A, SimdOpKind::ReplaceLaneI16x8),
            ("i32x4.replace_lane", 0x1C, SimdOpKind::ReplaceLaneI32x4),
            ("i64x2.extract_lane", 0x1D, SimdOpKind::ExtractLaneI64x2),
            ("i64x2.replace_lane", 0x1E, SimdOpKind::ReplaceLaneI64x2),
            ("f32x4.extract_lane", 0x1F, SimdOpKind::ExtractLaneF32x4),
            ("f32x4.replace_lane", 0x20, SimdOpKind::ReplaceLaneF32x4),
            ("f64x2.extract_lane", 0x21, SimdOpKind::ExtractLaneF64x2),
            ("f64x2.replace_lane", 0x22, SimdOpKind::ReplaceLaneF64x2),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_f32x4_arith3_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR19: `f32x4.abs` (0xE0), `f32x4.mul` (0xE6),
        // `f32x4.min` (0xE8) -- fetched live from BinarySIMD.md, the
        // FIRST genuine floating-point ARITHMETIC ops in this table
        // (PR17's f32x4/f64x2 splats were pure bit-pattern broadcasts,
        // no arithmetic). `f32x4.min` in particular is NOT the same
        // as Rust's `f32::min()` -- see `SimdOpKind::MinF32x4`'s own
        // doc comment for the NaN-propagation/signed-zero tie-break
        // semantics this crate mirrors from its own scalar `f32.min`
        // (0x96) handler.
        for (name, sub_opcode, kind) in [
            ("f32x4.abs", 0xE0, SimdOpKind::AbsF32x4),
            ("f32x4.mul", 0xE6, SimdOpKind::MulF32x4),
            ("f32x4.min", 0xE8, SimdOpKind::MinF32x4),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_f32x4_arith_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR29 (task #202-204): f32x4.neg (0xE1), f32x4.sqrt
        // (0xE3), f32x4.add (0xE4), f32x4.sub (0xE5), f32x4.div (0xE7) --
        // closes the last remaining gap in f32x4's core arithmetic family
        // (abs/mul/min landed in PR19 above). Each sub-opcode fetched
        // live from BinarySIMD.md and cross-checked against the
        // already-implemented f32x4.abs/mul/min entries -- see this
        // table's own doc comment for the full gap analysis (0xE2 is
        // genuinely unassigned in the spec, not a skipped op).
        for (name, sub_opcode, kind) in [
            ("f32x4.neg", 0xE1, SimdOpKind::NegF32x4),
            ("f32x4.sqrt", 0xE3, SimdOpKind::SqrtF32x4),
            ("f32x4.add", 0xE4, SimdOpKind::AddF32x4),
            ("f32x4.sub", 0xE5, SimdOpKind::SubF32x4),
            ("f32x4.div", 0xE7, SimdOpKind::DivF32x4),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_f32x4_cmp_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR30 (task #205-207): f32x4.eq (0x41), f32x4.ne
        // (0x42), f32x4.lt (0x43), f32x4.gt (0x44), f32x4.le (0x45),
        // f32x4.ge (0x46) -- fetched live from BinarySIMD.md, the f32x4
        // comparison family, mirroring the already-implemented i32x4/
        // i16x8/i8x16/i64x2 comparison families' boolean-mask shape.
        for (name, sub_opcode, kind) in [
            ("f32x4.eq", 0x41, SimdOpKind::EqF32x4),
            ("f32x4.ne", 0x42, SimdOpKind::NeF32x4),
            ("f32x4.lt", 0x43, SimdOpKind::LtF32x4),
            ("f32x4.gt", 0x44, SimdOpKind::GtF32x4),
            ("f32x4.le", 0x45, SimdOpKind::LeF32x4),
            ("f32x4.ge", 0x46, SimdOpKind::GeF32x4),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i32x4_f32x4_conversion_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR20 (task #177-179): `i32x4.trunc_sat_f32x4_s`
        // (0xF8), `i32x4.trunc_sat_f32x4_u` (0xF9), `f32x4.convert_i32x4_s`
        // (0xFA), `f32x4.convert_i32x4_u` (0xFB) -- fetched live from
        // BinarySIMD.md, this table's first `i32x4`<->`f32x4`
        // CONVERSION ops (a lane TYPE change, not just a value change
        // within one lane type, unlike every prior `f32x4` addition).
        // `trunc_sat_f32x4_s`/`_u` NEVER trap (NaN saturates to 0,
        // out-of-range saturates to the target bound) -- see
        // `SimdOpKind::TruncSatF32x4S`'s own doc comment for why this is
        // deliberately NOT the same trapping behavior as this crate's
        // scalar `i32.trunc_f32_s`/`_u` MVP opcodes.
        for (name, sub_opcode, kind) in [
            ("i32x4.trunc_sat_f32x4_s", 0xF8, SimdOpKind::TruncSatF32x4S),
            ("i32x4.trunc_sat_f32x4_u", 0xF9, SimdOpKind::TruncSatF32x4U),
            ("f32x4.convert_i32x4_s", 0xFA, SimdOpKind::ConvertI32x4S),
            ("f32x4.convert_i32x4_u", 0xFB, SimdOpKind::ConvertI32x4U),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i64x2_from_i32x4_widening_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR21 (task #180-182): `i64x2.extmul_low_i32x4_s`
        // (0xDC), `i64x2.extmul_high_i32x4_s` (0xDD),
        // `i64x2.extmul_low_i32x4_u` (0xDE), `i64x2.extmul_high_i32x4_u`
        // (0xDF) -- fetched live from BinarySIMD.md, cross-checked against
        // the already-implemented `i32x4.extmul_low_i16x8_s` (0xBC)/
        // `i64x2.abs` (0xC0)/`i64x2.ge_s` (0xDB) entries (all three
        // matched exactly). Completes the third and final rung of this
        // table's "extmul" widening-multiply family (i8x16->i16x8,
        // i16x8->i32x4, and now i32x4->i64x2). No `i64x2.dot_i32x4_s` --
        // same as the i16x8->i8x16 rung, WASM SIMD does not define a
        // dot-product for this lane-width pair.
        for (name, sub_opcode, kind) in [
            ("i64x2.extmul_low_i32x4_s", 0xDC, SimdOpKind::ExtmulLowI64x2S),
            ("i64x2.extmul_high_i32x4_s", 0xDD, SimdOpKind::ExtmulHighI64x2S),
            ("i64x2.extmul_low_i32x4_u", 0xDE, SimdOpKind::ExtmulLowI64x2U),
            ("i64x2.extmul_high_i32x4_u", 0xDF, SimdOpKind::ExtmulHighI64x2U),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
        assert!(get_simd_op_by_name("i64x2.dot_i32x4_s").is_none(), "WASM SIMD defines no i64x2.dot_i32x4_s");
    }

    #[test]
    fn simd_i16x8_q15mulr_sat_s_has_the_real_verified_sub_opcode_value() {
        // SIMD widen PR22 (task #183-185): `i16x8.q15mulr_sat_s` (0x82) --
        // fetched live from BinarySIMD.md, cross-checked against the
        // already-implemented `i16x8.neg` (0x81) and `i16x8.all_true`
        // (0x83) entries, which straddle it on either side -- 0x82 was the
        // one gap in that run and is not used by any other SIMD_OPS entry.
        // A genuinely new op family: a Q15 fixed-point ROUNDING SATURATING
        // multiply, not a plain wrapping/compare/min-max op like every
        // other i16x8 binary entry in this table.
        let op = get_simd_op(0x82).expect("0x82 should be i16x8.q15mulr_sat_s");
        assert_eq!(op.name, "i16x8.q15mulr_sat_s");
        assert_eq!(op.kind, SimdOpKind::Q15mulrSatI16x8S);
        assert_eq!(get_simd_op_by_name("i16x8.q15mulr_sat_s").map(|o| o.sub_opcode), Some(0x82));
    }

    #[test]
    fn simd_i32x4_trunc_sat_f64x2_zero_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR25 (task #190-192): `i32x4.trunc_sat_f64x2_s_zero`
        // (0xFC) / `i32x4.trunc_sat_f64x2_u_zero` (0xFD) -- fetched live
        // from BinarySIMD.md, cross-checked against the already-
        // implemented `i32x4.trunc_sat_f32x4_s`/`_u` (0xF8/0xF9) and
        // `f32x4.convert_i32x4_s`/`_u` (0xFA/0xFB) entries: all four
        // matched exactly, confirming 0xFC/0xFD sit immediately past
        // that conversion family with no gap.
        for (name, sub_opcode, kind) in [
            ("i32x4.trunc_sat_f64x2_s_zero", 0xFC, SimdOpKind::TruncSatF64x2SZero),
            ("i32x4.trunc_sat_f64x2_u_zero", 0xFD, SimdOpKind::TruncSatF64x2UZero),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_extend_low_high_widening_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR26 (task #193-195): i16x8.extend_low/high_i8x16_s/_u
        // (0x87/0x88/0x89/0x8A) and i32x4.extend_low/high_i16x8_s/_u
        // (0xA7/0xA8/0xA9/0xAA) -- fetched live from BinarySIMD.md,
        // cross-checked against the already-implemented
        // i16x8.extmul_low_i8x16_s (0x9C)/i32x4.extmul_low_i16x8_s (0xBC)
        // entries (both matched exactly). First of three PRs needed to
        // unlock simd_conversions.wast (narrow, then promote/demote/
        // convert_low follow); no corpus vendoring happens until all 16
        // opcodes across those PRs are in.
        for (name, sub_opcode, kind) in [
            ("i16x8.extend_low_i8x16_s", 0x87, SimdOpKind::ExtendLowI8x16S),
            ("i16x8.extend_high_i8x16_s", 0x88, SimdOpKind::ExtendHighI8x16S),
            ("i16x8.extend_low_i8x16_u", 0x89, SimdOpKind::ExtendLowI8x16U),
            ("i16x8.extend_high_i8x16_u", 0x8A, SimdOpKind::ExtendHighI8x16U),
            ("i32x4.extend_low_i16x8_s", 0xA7, SimdOpKind::ExtendLowI16x8S),
            ("i32x4.extend_high_i16x8_s", 0xA8, SimdOpKind::ExtendHighI16x8S),
            ("i32x4.extend_low_i16x8_u", 0xA9, SimdOpKind::ExtendLowI16x8U),
            ("i32x4.extend_high_i16x8_u", 0xAA, SimdOpKind::ExtendHighI16x8U),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_i64x2_extend_low_high_widening_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR36 (task #223-225): i64x2.extend_low/high_i32x4_s/_u
        // (0xC7/0xC8/0xC9/0xCA) -- fetched live from BinarySIMD.md,
        // cross-checked against the already-implemented i64x2.bitmask
        // (0xC4)/i64x2.shl (0xCB) entries: 0xC7-0xCA sit in the gap
        // between them with no collision. THIRD and FINAL rung of the
        // "extend" family (i16x8/i32x4 both landed in PR26 above).
        for (name, sub_opcode, kind) in [
            ("i64x2.extend_low_i32x4_s", 0xC7, SimdOpKind::ExtendLowI32x4S),
            ("i64x2.extend_high_i32x4_s", 0xC8, SimdOpKind::ExtendHighI32x4S),
            ("i64x2.extend_low_i32x4_u", 0xC9, SimdOpKind::ExtendLowI32x4U),
            ("i64x2.extend_high_i32x4_u", 0xCA, SimdOpKind::ExtendHighI32x4U),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_narrow_saturating_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR27 (task #196-198): i8x16.narrow_i16x8_s/_u
        // (0x65/0x66) and i16x8.narrow_i32x4_s/_u (0x85/0x86) -- fetched
        // live from BinarySIMD.md, cross-checked against the already-
        // implemented i8x16.bitmask (0x64)/i16x8.all_true (0x83)/
        // i16x8.bitmask (0x84) entries: 0x65/0x66 sit immediately past
        // i8x16.bitmask's 0x64 with no gap, 0x85/0x86 sit immediately
        // past i16x8.bitmask's 0x84 with no gap. Second of three PRs
        // needed to unlock simd_conversions.wast (extend done in PR26,
        // promote/demote/convert_low to follow); no corpus vendoring
        // happens until all 16 opcodes across those PRs are in.
        for (name, sub_opcode, kind) in [
            ("i8x16.narrow_i16x8_s", 0x65, SimdOpKind::NarrowI16x8S),
            ("i8x16.narrow_i16x8_u", 0x66, SimdOpKind::NarrowI16x8U),
            ("i16x8.narrow_i32x4_s", 0x85, SimdOpKind::NarrowI32x4S),
            ("i16x8.narrow_i32x4_u", 0x86, SimdOpKind::NarrowI32x4U),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_promote_demote_convert_low_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR28 (task #199-201): f32x4.demote_f64x2_zero
        // (0x5E), f64x2.promote_low_f32x4 (0x5F), f64x2.convert_low_i32x4_s
        // (0xFE), f64x2.convert_low_i32x4_u (0xFF) -- fetched live from
        // BinarySIMD.md. Third and FINAL of three PRs needed to unlock
        // simd_conversions.wast (extend done in PR26, narrow done in
        // PR27, promote/demote/convert_low here) -- all 16 opcodes now
        // exist.
        for (name, sub_opcode, kind) in [
            ("f32x4.demote_f64x2_zero", 0x5E, SimdOpKind::DemoteF64x2Zero),
            ("f64x2.promote_low_f32x4", 0x5F, SimdOpKind::PromoteLowF32x4),
            ("f64x2.convert_low_i32x4_s", 0xFE, SimdOpKind::ConvertLowI32x4S),
            ("f64x2.convert_low_i32x4_u", 0xFF, SimdOpKind::ConvertLowI32x4U),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
    }

    #[test]
    fn simd_f64x2_arith_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR31 (task #208-210): f64x2.neg (0xED), f64x2.sqrt
        // (0xEF), f64x2.add (0xF0), f64x2.sub (0xF1), f64x2.mul (0xF2),
        // f64x2.div (0xF3) -- fetched live from BinarySIMD.md and
        // cross-checked against the still-unimplemented f64x2.abs
        // (0xEC, immediately before this run) and f64x2.min/max (0xF4/
        // 0xF5, immediately after), confirming this run is exactly
        // 0xED, 0xEF-0xF3 with a real gap at 0xEE -- a direct structural
        // mirror of PR29's f32x4.neg/sqrt/add/sub/div, at f64x2's
        // 2-lane width, plus `mul` riding along on the same binary-op
        // shape (f64x2.mul didn't exist before this PR).
        for (name, sub_opcode, kind) in [
            ("f64x2.neg", 0xED, SimdOpKind::NegF64x2),
            ("f64x2.sqrt", 0xEF, SimdOpKind::SqrtF64x2),
            ("f64x2.add", 0xF0, SimdOpKind::AddF64x2),
            ("f64x2.sub", 0xF1, SimdOpKind::SubF64x2),
            ("f64x2.mul", 0xF2, SimdOpKind::MulF64x2),
            ("f64x2.div", 0xF3, SimdOpKind::DivF64x2),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
        // 0xEE is a real gap (unassigned in the SIMD proposal's own
        // binary encoding), not a placeholder for a future op this PR
        // is skipping -- same shape as f32x4's own 0xE2 gap.
        assert!(get_simd_op(0xEE).is_none(), "0xEE is an unassigned gap between f64x2.neg and f64x2.sqrt");
    }

    #[test]
    fn simd_f64x2_cmp_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR32 (task #211-213): f64x2.eq (0x47), f64x2.ne
        // (0x48), f64x2.lt (0x49), f64x2.gt (0x4A), f64x2.le (0x4B),
        // f64x2.ge (0x4C) -- fetched live from BinarySIMD.md, a direct
        // structural mirror of PR30's f32x4 comparison family
        // (0x41-0x46), just at f64x2's 2-lane width.
        for (name, sub_opcode, kind) in [
            ("f64x2.eq", 0x47, SimdOpKind::EqF64x2),
            ("f64x2.ne", 0x48, SimdOpKind::NeF64x2),
            ("f64x2.lt", 0x49, SimdOpKind::LtF64x2),
            ("f64x2.gt", 0x4A, SimdOpKind::GtF64x2),
            ("f64x2.le", 0x4B, SimdOpKind::LeF64x2),
            ("f64x2.ge", 0x4C, SimdOpKind::GeF64x2),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
        // 0x41-0x46 (the already-implemented f32x4 comparison family,
        // PR30) precede this run with no overlap, and v128.not (0x4D)
        // sits immediately past it, confirming 0x47-0x4C are genuinely
        // free.
        for f32x4_sub_opcode in 0x41u32..=0x46u32 {
            let op = get_simd_op(f32x4_sub_opcode).unwrap();
            assert!(op.name.starts_with("f32x4."), "0x{f32x4_sub_opcode:02x} should still be an f32x4 op, got {}", op.name);
        }
        assert_eq!(get_simd_op(0x4D).map(|o| o.name), Some("v128.not"));
    }

    #[test]
    fn simd_sat_add_sub_family_has_the_real_verified_sub_opcode_values() {
        // SIMD widen PR33 (task #214-216): i8x16.add_sat_s (0x6F),
        // i8x16.add_sat_u (0x70), i8x16.sub_sat_s (0x72),
        // i8x16.sub_sat_u (0x73), i16x8.add_sat_s (0x8F),
        // i16x8.add_sat_u (0x90), i16x8.sub_sat_s (0x92),
        // i16x8.sub_sat_u (0x93) -- fetched live from BinarySIMD.md and
        // cross-checked against the already-implemented i8x16.add
        // (0x6E)/i8x16.sub (0x71)/i16x8.add (0x8E)/i16x8.sub (0x91)
        // entries.
        for (name, sub_opcode, kind) in [
            ("i8x16.add_sat_s", 0x6F, SimdOpKind::AddSatI8x16S),
            ("i8x16.add_sat_u", 0x70, SimdOpKind::AddSatI8x16U),
            ("i8x16.sub_sat_s", 0x72, SimdOpKind::SubSatI8x16S),
            ("i8x16.sub_sat_u", 0x73, SimdOpKind::SubSatI8x16U),
            ("i16x8.add_sat_s", 0x8F, SimdOpKind::AddSatI16x8S),
            ("i16x8.add_sat_u", 0x90, SimdOpKind::AddSatI16x8U),
            ("i16x8.sub_sat_s", 0x92, SimdOpKind::SubSatI16x8S),
            ("i16x8.sub_sat_u", 0x93, SimdOpKind::SubSatI16x8U),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
        // i8x16.add (0x6E) and i8x16.sub (0x71) bracket the 0x6F/0x70
        // pair with no overlap; i16x8.add (0x8E) and i16x8.sub (0x91)
        // bracket the 0x8F/0x90 pair the same way.
        assert_eq!(get_simd_op(0x6E).map(|o| o.name), Some("i8x16.add"));
        assert_eq!(get_simd_op(0x71).map(|o| o.name), Some("i8x16.sub"));
        assert_eq!(get_simd_op(0x8E).map(|o| o.name), Some("i16x8.add"));
        assert_eq!(get_simd_op(0x91).map(|o| o.name), Some("i16x8.sub"));
    }

    #[test]
    fn simd_f32x4_max_pmin_pmax_have_the_real_verified_sub_opcode_values() {
        // SIMD widen PR34 (task #217-219): f32x4.max (0xE9), f32x4.pmin
        // (0xEA), f32x4.pmax (0xEB) -- fetched live from BinarySIMD.md and
        // cross-checked against the already-implemented f32x4.min (0xE8)
        // entry: this run sits immediately past it with no gap.
        for (name, sub_opcode, kind) in [
            ("f32x4.max", 0xE9, SimdOpKind::MaxF32x4),
            ("f32x4.pmin", 0xEA, SimdOpKind::PminF32x4),
            ("f32x4.pmax", 0xEB, SimdOpKind::PmaxF32x4),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
        // f32x4.min (0xE8) immediately precedes this run with no overlap.
        assert_eq!(get_simd_op(0xE8).map(|o| o.name), Some("f32x4.min"));
    }

    #[test]
    fn simd_f64x2_abs_min_max_pmin_pmax_have_the_real_verified_sub_opcode_values() {
        // SIMD widen PR35 (task #220-222): f64x2.abs (0xEC), f64x2.min
        // (0xF4), f64x2.max (0xF5), f64x2.pmin (0xF6), f64x2.pmax (0xF7)
        // -- fetched live from BinarySIMD.md and cross-checked against
        // the already-implemented f64x2.neg (0xED) and f64x2.div (0xF3)
        // entries: 0xEC sits immediately before f64x2.neg's 0xED, and
        // 0xF4-0xF7 sit immediately past f64x2.div's 0xF3 with no gap.
        for (name, sub_opcode, kind) in [
            ("f64x2.abs", 0xEC, SimdOpKind::AbsF64x2),
            ("f64x2.min", 0xF4, SimdOpKind::MinF64x2),
            ("f64x2.max", 0xF5, SimdOpKind::MaxF64x2),
            ("f64x2.pmin", 0xF6, SimdOpKind::PminF64x2),
            ("f64x2.pmax", 0xF7, SimdOpKind::PmaxF64x2),
        ] {
            let op = get_simd_op(sub_opcode).unwrap_or_else(|| panic!("{sub_opcode:#04x} should be {name}"));
            assert_eq!(op.name, name);
            assert_eq!(op.kind, kind);
            assert_eq!(get_simd_op_by_name(name).map(|o| o.sub_opcode), Some(sub_opcode));
        }
        // f64x2.neg (0xED) immediately follows f64x2.abs (0xEC) with no
        // overlap; f64x2.div (0xF3) immediately precedes f64x2.min (0xF4)
        // with no overlap.
        assert_eq!(get_simd_op(0xED).map(|o| o.name), Some("f64x2.neg"));
        assert_eq!(get_simd_op(0xF3).map(|o| o.name), Some("f64x2.div"));
    }
}
