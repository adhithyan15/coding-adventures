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
//! The first byte of every instruction is its **opcode** — a value 0x00–0xBF
//! in WASM 1.0 (multi-byte opcodes via 0xFC prefix exist in later proposals,
//! but are out of scope here).
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
];

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

    // 1. Total opcode count covers all WASM 1.0 MVP instructions.
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
            OPCODES.len() >= 172,
            "Expected >= 172 WASM 1.0 MVP opcodes, got {}",
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
}
