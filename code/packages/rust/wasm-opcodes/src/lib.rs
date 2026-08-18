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
    fn simd_ops_table_has_the_expected_81_entries_and_no_duplicates() {
        assert_eq!(SIMD_OPS.len(), 81);

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
}
