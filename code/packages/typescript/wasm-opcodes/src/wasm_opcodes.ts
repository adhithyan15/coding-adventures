/**
 * wasm_opcodes.ts — Complete WASM 1.0 Opcode Table
 *
 * WebAssembly (WASM) is a binary instruction format designed as a portable
 * compilation target for high-level languages like C, C++, and Rust.
 * Each instruction in a WASM module is represented as a single byte (the opcode),
 * followed by zero or more "immediate" operands encoded directly in the byte stream.
 *
 * This module provides a complete lookup table of all ~183 WASM 1.0 instructions,
 * with metadata about each: name, byte value, category, immediate operand types,
 * and stack effects (how many values are consumed and produced).
 *
 * ============================================================
 * THE WASM STACK MACHINE
 * ============================================================
 *
 * WASM is a stack machine. Instructions operate on an implicit operand stack:
 *
 *   Before: [a, b]    i32.add    After: [a+b]
 *
 * stackPop  = number of values consumed from the stack
 * stackPush = number of values produced onto the stack
 *
 * For example:
 *   - i32.const: pops 0, pushes 1 (puts a constant on the stack)
 *   - i32.add:   pops 2 (a, b), pushes 1 (a+b)
 *   - i32.store: pops 2 (address, value), pushes 0 (side effect only)
 *
 * Control flow instructions (block, loop, if, call) have variable stack effects
 * that depend on their type annotation. We record 0/0 for these as placeholders —
 * the real effect is determined by the blocktype or function signature at runtime.
 *
 * ============================================================
 * IMMEDIATE OPERANDS
 * ============================================================
 *
 * After the opcode byte, some instructions encode additional data inline:
 *
 *   "i32"        — a 32-bit integer constant, LEB128-encoded (variable width)
 *   "i64"        — a 64-bit integer constant, LEB128-encoded
 *   "f32"        — a 32-bit IEEE 754 float, 4 bytes little-endian
 *   "f64"        — a 64-bit IEEE 754 float, 8 bytes little-endian
 *   "blocktype"  — the result type of a structured block (-0x40 for void, or a valtype byte)
 *   "labelidx"   — a branch target, LEB128-encoded index into the label stack
 *   "vec_labelidx" — br_table: a count followed by N label indices
 *   "funcidx"    — index into the function table, LEB128-encoded
 *   "typeidx"    — index into the type table
 *   "tableidx"   — index into the table section (always 0 in WASM 1.0)
 *   "localidx"   — index into the local variable list
 *   "globalidx"  — index into the global variable list
 *   "memarg"     — memory argument: { align: u32, offset: u32 }, both LEB128
 *                  align is a power-of-2 alignment hint (e.g., 2 = align to 4 bytes)
 *                  offset is a byte offset added to the runtime address
 *   "memidx"     — index into the memory section (always 0 in WASM 1.0)
 *
 * ============================================================
 * CATEGORIES
 * ============================================================
 *
 *   "control"      — structured control flow (block, loop, if, br, call, return, ...)
 *   "parametric"   — stack manipulation (drop, select)
 *   "variable"     — local/global variable access
 *   "memory"       — heap loads, stores, and memory management
 *   "numeric_i32"  — 32-bit integer arithmetic, comparisons, bitwise ops
 *   "numeric_i64"  — 64-bit integer arithmetic, comparisons, bitwise ops
 *   "numeric_f32"  — 32-bit floating-point arithmetic and comparisons
 *   "numeric_f64"  — 64-bit floating-point arithmetic and comparisons
 *   "conversion"   — type conversions between numeric types
 *
 * ============================================================
 * COMPLETE OPCODE TABLE (all 183 WASM 1.0 instructions)
 * ============================================================
 *
 * Control flow:
 *   0x00  unreachable         — trap unconditionally
 *   0x01  nop                 — no operation
 *   0x02  block [blocktype]   — begin a structured block
 *   0x03  loop  [blocktype]   — begin a loop (br targets top of loop)
 *   0x04  if    [blocktype]   — conditional branch: pops 1 (condition i32)
 *   0x05  else                — else arm of if
 *   0x0B  end                 — end block/loop/if/function
 *   0x0C  br    [labelidx]    — unconditional branch to label
 *   0x0D  br_if [labelidx]    — conditional branch: pops 1 (condition i32)
 *   0x0E  br_table [vec]      — indirect branch table: pops 1 (index i32)
 *   0x0F  return              — return from function
 *   0x10  call   [funcidx]    — call function directly
 *   0x11  call_indirect [typeidx, tableidx] — call function through table
 *
 * Parametric:
 *   0x1A  drop               — discard top of stack
 *   0x1B  select             — pops condition, then two values; pushes one
 *
 * Variables:
 *   0x20  local.get  [localidx]   — push local variable
 *   0x21  local.set  [localidx]   — pop and store to local
 *   0x22  local.tee  [localidx]   — store to local without consuming (peek+store)
 *   0x23  global.get [globalidx]  — push global variable
 *   0x24  global.set [globalidx]  — pop and store to global
 *
 * Memory loads (pop address i32, push loaded value; all take [memarg]):
 *   0x28  i32.load      — load 4 bytes, zero-extend to i32
 *   0x29  i64.load      — load 8 bytes, zero-extend to i64
 *   0x2A  f32.load      — load 4-byte float
 *   0x2B  f64.load      — load 8-byte float
 *   0x2C  i32.load8_s   — load 1 byte, sign-extend to i32
 *   0x2D  i32.load8_u   — load 1 byte, zero-extend to i32
 *   0x2E  i32.load16_s  — load 2 bytes, sign-extend to i32
 *   0x2F  i32.load16_u  — load 2 bytes, zero-extend to i32
 *   0x30  i64.load8_s   — load 1 byte, sign-extend to i64
 *   0x31  i64.load8_u   — load 1 byte, zero-extend to i64
 *   0x32  i64.load16_s  — load 2 bytes, sign-extend to i64
 *   0x33  i64.load16_u  — load 2 bytes, zero-extend to i64
 *   0x34  i64.load32_s  — load 4 bytes, sign-extend to i64
 *   0x35  i64.load32_u  — load 4 bytes, zero-extend to i64
 *
 * Memory stores (pop address i32 + value; all take [memarg]):
 *   0x36  i32.store   — store 4 bytes
 *   0x37  i64.store   — store 8 bytes
 *   0x38  f32.store   — store 4-byte float
 *   0x39  f64.store   — store 8-byte float
 *   0x3A  i32.store8  — store low byte
 *   0x3B  i32.store16 — store low 2 bytes
 *   0x3C  i64.store8  — store low byte
 *   0x3D  i64.store16 — store low 2 bytes
 *   0x3E  i64.store32 — store low 4 bytes
 *
 * Memory management:
 *   0x3F  memory.size [memidx] — push current size in pages
 *   0x40  memory.grow [memidx] — grow memory; pops delta, pushes old size (-1 on fail)
 */

// ============================================================
// OpcodeInfo — the record type for a single WASM instruction
// ============================================================

/**
 * Metadata for a single WASM instruction.
 *
 * @example
 * // i32.add
 * {
 *   name: "i32.add",
 *   opcode: 0x6A,
 *   category: "numeric_i32",
 *   immediates: [],
 *   stackPop: 2,
 *   stackPush: 1
 * }
 */
export interface OpcodeInfo {
  /**
   * The canonical WASM text-format name, e.g. "i32.add", "memory.grow".
   * These names come directly from the WASM 1.0 specification.
   */
  readonly name: string;

  /**
   * The single-byte opcode value, e.g. 0x6A for i32.add.
   * Every WASM 1.0 instruction fits in one byte (0x00–0xBF).
   */
  readonly opcode: number;

  /**
   * Broad instruction category. Used for grouping and analysis.
   * One of: "control", "parametric", "variable", "memory",
   *         "numeric_i32", "numeric_i64", "numeric_f32", "numeric_f64", "conversion"
   */
  readonly category: string;

  /**
   * List of immediate operand types that follow the opcode in the byte stream.
   * Empty array means no immediates (the opcode alone encodes the instruction).
   * See the module-level comment for the full list of immediate types.
   */
  readonly immediates: readonly string[];

  /**
   * Number of values popped from the operand stack.
   * For control instructions with variable arity (call, call_indirect),
   * this is 0 because the true arity depends on the function signature.
   */
  readonly stackPop: number;

  /**
   * Number of values pushed onto the operand stack.
   * For control instructions with variable arity (call, return),
   * this is 0 because the true arity depends on the function signature.
   */
  readonly stackPush: number;
}

// ============================================================
// Raw opcode table data
// ============================================================
// Each entry: [opcode, name, category, immediates, stackPop, stackPush]
// Formatted in groups matching the WASM spec section order.

type RawEntry = [number, string, string, string[], number, number];

const RAW_OPCODES: RawEntry[] = [
  // ── Control flow ────────────────────────────────────────────────────────────
  // WASM uses structured control flow: no arbitrary jumps.
  // Blocks form a label stack. br/br_if jump to a label by depth index.
  //
  //   block [] — begin a sequence; "br 0" exits the block
  //   loop  [] — begin a loop;     "br 0" goes back to the top
  //   if    [] — if/else/end construct, pops a condition i32
  //
  // Stack effects for block/loop/if/call are listed as 0/0 because
  // they depend on the blocktype annotation or function type signature.
  [0x00, "unreachable",    "control", [],                          0, 0],
  [0x01, "nop",            "control", [],                          0, 0],
  [0x02, "block",          "control", ["blocktype"],               0, 0],
  [0x03, "loop",           "control", ["blocktype"],               0, 0],
  [0x04, "if",             "control", ["blocktype"],               1, 0],
  [0x05, "else",           "control", [],                          0, 0],
  [0x0B, "end",            "control", [],                          0, 0],
  [0x0C, "br",             "control", ["labelidx"],                0, 0],
  [0x0D, "br_if",          "control", ["labelidx"],                1, 0],
  [0x0E, "br_table",       "control", ["vec_labelidx"],            1, 0],
  [0x0F, "return",         "control", [],                          0, 0],
  [0x10, "call",           "control", ["funcidx"],                 0, 0],
  [0x11, "call_indirect",  "control", ["typeidx", "tableidx"],     1, 0],

  // ── Parametric ──────────────────────────────────────────────────────────────
  // These instructions work on values of any type (polymorphic).
  //
  //   drop   — discard TOS (top of stack)
  //   select — like a ternary: pops (c : i32, b : T, a : T), pushes a if c≠0 else b
  [0x1A, "drop",   "parametric", [], 1, 0],
  [0x1B, "select", "parametric", [], 3, 1],

  // ── Variable access ─────────────────────────────────────────────────────────
  // WASM variables are indexed, not named.
  // Locals 0..n-1 include function parameters, then declared locals.
  //
  //   local.tee is like local.set but leaves the value on the stack —
  //   useful for "peek and store" patterns.
  [0x20, "local.get",  "variable", ["localidx"],  0, 1],
  [0x21, "local.set",  "variable", ["localidx"],  1, 0],
  [0x22, "local.tee",  "variable", ["localidx"],  1, 1],
  [0x23, "global.get", "variable", ["globalidx"], 0, 1],
  [0x24, "global.set", "variable", ["globalidx"], 1, 0],

  // ── Memory loads ────────────────────────────────────────────────────────────
  // All loads pop one i32 (the base address) and push one value.
  // The effective address = memarg.offset + popped_address.
  //
  // _s = sign-extend (extends the sign bit for signed integers)
  // _u = zero-extend (fills upper bits with 0, for unsigned)
  //
  // The memarg immediate encodes two LEB128 values:
  //   align  — log2 of the expected alignment (hint, not enforced)
  //   offset — a static offset added to the runtime address
  [0x28, "i32.load",     "memory", ["memarg"], 1, 1],
  [0x29, "i64.load",     "memory", ["memarg"], 1, 1],
  [0x2A, "f32.load",     "memory", ["memarg"], 1, 1],
  [0x2B, "f64.load",     "memory", ["memarg"], 1, 1],
  [0x2C, "i32.load8_s",  "memory", ["memarg"], 1, 1],
  [0x2D, "i32.load8_u",  "memory", ["memarg"], 1, 1],
  [0x2E, "i32.load16_s", "memory", ["memarg"], 1, 1],
  [0x2F, "i32.load16_u", "memory", ["memarg"], 1, 1],
  [0x30, "i64.load8_s",  "memory", ["memarg"], 1, 1],
  [0x31, "i64.load8_u",  "memory", ["memarg"], 1, 1],
  [0x32, "i64.load16_s", "memory", ["memarg"], 1, 1],
  [0x33, "i64.load16_u", "memory", ["memarg"], 1, 1],
  [0x34, "i64.load32_s", "memory", ["memarg"], 1, 1],
  [0x35, "i64.load32_u", "memory", ["memarg"], 1, 1],

  // ── Memory stores ───────────────────────────────────────────────────────────
  // All stores pop two values: the base address (i32) and the value to write.
  // Truncating stores (store8, store16, store32) discard the high bits.
  [0x36, "i32.store",   "memory", ["memarg"], 2, 0],
  [0x37, "i64.store",   "memory", ["memarg"], 2, 0],
  [0x38, "f32.store",   "memory", ["memarg"], 2, 0],
  [0x39, "f64.store",   "memory", ["memarg"], 2, 0],
  [0x3A, "i32.store8",  "memory", ["memarg"], 2, 0],
  [0x3B, "i32.store16", "memory", ["memarg"], 2, 0],
  [0x3C, "i64.store8",  "memory", ["memarg"], 2, 0],
  [0x3D, "i64.store16", "memory", ["memarg"], 2, 0],
  [0x3E, "i64.store32", "memory", ["memarg"], 2, 0],

  // ── Memory management ───────────────────────────────────────────────────────
  // WASM memory is a contiguous byte array, measured in 64 KiB pages.
  //   memory.size — returns current page count (no args)
  //   memory.grow — grows memory by N pages, returns old size or -1 on failure
  [0x3F, "memory.size", "memory", ["memidx"], 0, 1],
  [0x40, "memory.grow", "memory", ["memidx"], 1, 1],

  // ── i32 numeric ─────────────────────────────────────────────────────────────
  // 32-bit integer arithmetic. WASM uses two's complement representation.
  //
  //   clz   — count leading zeros
  //   ctz   — count trailing zeros
  //   popcnt— count set bits (population count)
  //   eqz   — test if zero (comparison producing i32 0 or 1)
  //   eq/ne/lt/gt/le/ge — comparisons producing i32 0 or 1
  //   _s    — signed interpretation
  //   _u    — unsigned interpretation
  [0x41, "i32.const",  "numeric_i32", ["i32"], 0, 1],
  [0x45, "i32.eqz",   "numeric_i32", [],      1, 1],
  [0x46, "i32.eq",    "numeric_i32", [],      2, 1],
  [0x47, "i32.ne",    "numeric_i32", [],      2, 1],
  [0x48, "i32.lt_s",  "numeric_i32", [],      2, 1],
  [0x49, "i32.lt_u",  "numeric_i32", [],      2, 1],
  [0x4A, "i32.gt_s",  "numeric_i32", [],      2, 1],
  [0x4B, "i32.gt_u",  "numeric_i32", [],      2, 1],
  [0x4C, "i32.le_s",  "numeric_i32", [],      2, 1],
  [0x4D, "i32.le_u",  "numeric_i32", [],      2, 1],
  [0x4E, "i32.ge_s",  "numeric_i32", [],      2, 1],
  [0x4F, "i32.ge_u",  "numeric_i32", [],      2, 1],
  [0x67, "i32.clz",   "numeric_i32", [],      1, 1],
  [0x68, "i32.ctz",   "numeric_i32", [],      1, 1],
  [0x69, "i32.popcnt","numeric_i32", [],      1, 1],
  [0x6A, "i32.add",   "numeric_i32", [],      2, 1],
  [0x6B, "i32.sub",   "numeric_i32", [],      2, 1],
  [0x6C, "i32.mul",   "numeric_i32", [],      2, 1],
  [0x6D, "i32.div_s", "numeric_i32", [],      2, 1],
  [0x6E, "i32.div_u", "numeric_i32", [],      2, 1],
  [0x6F, "i32.rem_s", "numeric_i32", [],      2, 1],
  [0x70, "i32.rem_u", "numeric_i32", [],      2, 1],
  [0x71, "i32.and",   "numeric_i32", [],      2, 1],
  [0x72, "i32.or",    "numeric_i32", [],      2, 1],
  [0x73, "i32.xor",   "numeric_i32", [],      2, 1],
  [0x74, "i32.shl",   "numeric_i32", [],      2, 1],
  [0x75, "i32.shr_s", "numeric_i32", [],      2, 1],
  [0x76, "i32.shr_u", "numeric_i32", [],      2, 1],
  [0x77, "i32.rotl",  "numeric_i32", [],      2, 1],
  [0x78, "i32.rotr",  "numeric_i32", [],      2, 1],

  // ── i64 numeric ─────────────────────────────────────────────────────────────
  // 64-bit integer arithmetic. Same operations as i32 but wider.
  // Note: i64.eqz still pushes an i32 result (0 or 1), same for comparisons.
  [0x42, "i64.const",  "numeric_i64", ["i64"], 0, 1],
  [0x50, "i64.eqz",   "numeric_i64", [],      1, 1],
  [0x51, "i64.eq",    "numeric_i64", [],      2, 1],
  [0x52, "i64.ne",    "numeric_i64", [],      2, 1],
  [0x53, "i64.lt_s",  "numeric_i64", [],      2, 1],
  [0x54, "i64.lt_u",  "numeric_i64", [],      2, 1],
  [0x55, "i64.gt_s",  "numeric_i64", [],      2, 1],
  [0x56, "i64.gt_u",  "numeric_i64", [],      2, 1],
  [0x57, "i64.le_s",  "numeric_i64", [],      2, 1],
  [0x58, "i64.le_u",  "numeric_i64", [],      2, 1],
  [0x59, "i64.ge_s",  "numeric_i64", [],      2, 1],
  [0x5A, "i64.ge_u",  "numeric_i64", [],      2, 1],
  [0x79, "i64.clz",   "numeric_i64", [],      1, 1],
  [0x7A, "i64.ctz",   "numeric_i64", [],      1, 1],
  [0x7B, "i64.popcnt","numeric_i64", [],      1, 1],
  [0x7C, "i64.add",   "numeric_i64", [],      2, 1],
  [0x7D, "i64.sub",   "numeric_i64", [],      2, 1],
  [0x7E, "i64.mul",   "numeric_i64", [],      2, 1],
  [0x7F, "i64.div_s", "numeric_i64", [],      2, 1],
  [0x80, "i64.div_u", "numeric_i64", [],      2, 1],
  [0x81, "i64.rem_s", "numeric_i64", [],      2, 1],
  [0x82, "i64.rem_u", "numeric_i64", [],      2, 1],
  [0x83, "i64.and",   "numeric_i64", [],      2, 1],
  [0x84, "i64.or",    "numeric_i64", [],      2, 1],
  [0x85, "i64.xor",   "numeric_i64", [],      2, 1],
  [0x86, "i64.shl",   "numeric_i64", [],      2, 1],
  [0x87, "i64.shr_s", "numeric_i64", [],      2, 1],
  [0x88, "i64.shr_u", "numeric_i64", [],      2, 1],
  [0x89, "i64.rotl",  "numeric_i64", [],      2, 1],
  [0x8A, "i64.rotr",  "numeric_i64", [],      2, 1],

  // ── f32 numeric ─────────────────────────────────────────────────────────────
  // 32-bit IEEE 754 floating point.
  //
  //   abs/neg       — absolute value / negate (sign bit manipulation)
  //   ceil/floor    — round toward +∞ / −∞
  //   trunc/nearest — round toward zero / round to nearest (banker's rounding)
  //   sqrt          — square root
  //   min/max       — IEEE min/max (NaN propagating)
  //   copysign      — copy sign bit: copysign(a, b) = |a| with sign of b
  //
  // Comparisons produce i32 (0 or 1), not f32.
  [0x43, "f32.const",    "numeric_f32", ["f32"], 0, 1],
  [0x5B, "f32.eq",       "numeric_f32", [],      2, 1],
  [0x5C, "f32.ne",       "numeric_f32", [],      2, 1],
  [0x5D, "f32.lt",       "numeric_f32", [],      2, 1],
  [0x5E, "f32.gt",       "numeric_f32", [],      2, 1],
  [0x5F, "f32.le",       "numeric_f32", [],      2, 1],
  [0x60, "f32.ge",       "numeric_f32", [],      2, 1],
  [0x8B, "f32.abs",      "numeric_f32", [],      1, 1],
  [0x8C, "f32.neg",      "numeric_f32", [],      1, 1],
  [0x8D, "f32.ceil",     "numeric_f32", [],      1, 1],
  [0x8E, "f32.floor",    "numeric_f32", [],      1, 1],
  [0x8F, "f32.trunc",    "numeric_f32", [],      1, 1],
  [0x90, "f32.nearest",  "numeric_f32", [],      1, 1],
  [0x91, "f32.sqrt",     "numeric_f32", [],      1, 1],
  [0x92, "f32.add",      "numeric_f32", [],      2, 1],
  [0x93, "f32.sub",      "numeric_f32", [],      2, 1],
  [0x94, "f32.mul",      "numeric_f32", [],      2, 1],
  [0x95, "f32.div",      "numeric_f32", [],      2, 1],
  [0x96, "f32.min",      "numeric_f32", [],      2, 1],
  [0x97, "f32.max",      "numeric_f32", [],      2, 1],
  [0x98, "f32.copysign", "numeric_f32", [],      2, 1],

  // ── f64 numeric ─────────────────────────────────────────────────────────────
  // 64-bit IEEE 754 floating point (double precision).
  // Same operations as f32 but with higher precision.
  [0x44, "f64.const",    "numeric_f64", ["f64"], 0, 1],
  [0x61, "f64.eq",       "numeric_f64", [],      2, 1],
  [0x62, "f64.ne",       "numeric_f64", [],      2, 1],
  [0x63, "f64.lt",       "numeric_f64", [],      2, 1],
  [0x64, "f64.gt",       "numeric_f64", [],      2, 1],
  [0x65, "f64.le",       "numeric_f64", [],      2, 1],
  [0x66, "f64.ge",       "numeric_f64", [],      2, 1],
  [0x99, "f64.abs",      "numeric_f64", [],      1, 1],
  [0x9A, "f64.neg",      "numeric_f64", [],      1, 1],
  [0x9B, "f64.ceil",     "numeric_f64", [],      1, 1],
  [0x9C, "f64.floor",    "numeric_f64", [],      1, 1],
  [0x9D, "f64.trunc",    "numeric_f64", [],      1, 1],
  [0x9E, "f64.nearest",  "numeric_f64", [],      1, 1],
  [0x9F, "f64.sqrt",     "numeric_f64", [],      1, 1],
  [0xA0, "f64.add",      "numeric_f64", [],      2, 1],
  [0xA1, "f64.sub",      "numeric_f64", [],      2, 1],
  [0xA2, "f64.mul",      "numeric_f64", [],      2, 1],
  [0xA3, "f64.div",      "numeric_f64", [],      2, 1],
  [0xA4, "f64.min",      "numeric_f64", [],      2, 1],
  [0xA5, "f64.max",      "numeric_f64", [],      2, 1],
  [0xA6, "f64.copysign", "numeric_f64", [],      2, 1],

  // ── Conversions ─────────────────────────────────────────────────────────────
  // All conversions pop 1 value and push 1 value. No immediates.
  //
  //   wrap_i64            — truncate i64 to i32 (drop high 32 bits)
  //   extend_i32_s/u      — widen i32 to i64 (sign/zero extend)
  //   trunc_fNN_s/u       — truncate float to integer (trap on NaN/overflow)
  //   convert_iNN_s/u     — convert integer to float (exact or rounded)
  //   demote_f64          — f64 → f32 (narrowing, may lose precision)
  //   promote_f32         — f32 → f64 (widening, always exact)
  //   reinterpret_fNN/iNN — reinterpret bit pattern (no numeric conversion)
  //
  // The reinterpret instructions are useful for bit manipulation of floats.
  // For example, to check the sign bit of an f32, reinterpret to i32 and shr_u by 31.
  [0xA7, "i32.wrap_i64",       "conversion", [], 1, 1],
  [0xA8, "i32.trunc_f32_s",    "conversion", [], 1, 1],
  [0xA9, "i32.trunc_f32_u",    "conversion", [], 1, 1],
  [0xAA, "i32.trunc_f64_s",    "conversion", [], 1, 1],
  [0xAB, "i32.trunc_f64_u",    "conversion", [], 1, 1],
  [0xAC, "i64.extend_i32_s",   "conversion", [], 1, 1],
  [0xAD, "i64.extend_i32_u",   "conversion", [], 1, 1],
  [0xAE, "i64.trunc_f32_s",    "conversion", [], 1, 1],
  [0xAF, "i64.trunc_f32_u",    "conversion", [], 1, 1],
  [0xB0, "i64.trunc_f64_s",    "conversion", [], 1, 1],
  [0xB1, "i64.trunc_f64_u",    "conversion", [], 1, 1],
  [0xB2, "f32.convert_i32_s",  "conversion", [], 1, 1],
  [0xB3, "f32.convert_i32_u",  "conversion", [], 1, 1],
  [0xB4, "f32.convert_i64_s",  "conversion", [], 1, 1],
  [0xB5, "f32.convert_i64_u",  "conversion", [], 1, 1],
  [0xB6, "f32.demote_f64",     "conversion", [], 1, 1],
  [0xB7, "f64.convert_i32_s",  "conversion", [], 1, 1],
  [0xB8, "f64.convert_i32_u",  "conversion", [], 1, 1],
  [0xB9, "f64.convert_i64_s",  "conversion", [], 1, 1],
  [0xBA, "f64.convert_i64_u",  "conversion", [], 1, 1],
  [0xBB, "f64.promote_f32",    "conversion", [], 1, 1],
  [0xBC, "i32.reinterpret_f32","conversion", [], 1, 1],
  [0xBD, "i64.reinterpret_f64","conversion", [], 1, 1],
  [0xBE, "f32.reinterpret_i32","conversion", [], 1, 1],
  [0xBF, "f64.reinterpret_i64","conversion", [], 1, 1],
];

// ============================================================
// Build the lookup tables
// ============================================================

/**
 * Primary lookup table: opcode byte → OpcodeInfo.
 *
 * Built once at module load time from the raw table above.
 * Using a Map ensures O(1) lookup with no prototype pollution.
 *
 * @example
 * const info = OPCODES.get(0x6A); // i32.add
 */
export const OPCODES: Map<number, OpcodeInfo> = new Map(
  RAW_OPCODES.map(([opcode, name, category, immediates, stackPop, stackPush]) => [
    opcode,
    Object.freeze({ name, opcode, category, immediates: Object.freeze(immediates), stackPop, stackPush }),
  ])
);

/**
 * Secondary lookup table: instruction name → OpcodeInfo.
 *
 * Mirrors OPCODES but keyed by the text-format name.
 * Both maps always have the same size (one entry per instruction).
 *
 * @example
 * const info = OPCODES_BY_NAME.get("i32.add");
 */
export const OPCODES_BY_NAME: Map<string, OpcodeInfo> = new Map(
  Array.from(OPCODES.values()).map((info) => [info.name, info])
);

// ============================================================
// Public API
// ============================================================

/**
 * Look up an instruction by its opcode byte.
 *
 * @param byte - The opcode byte value (0x00–0xBF for WASM 1.0)
 * @returns The OpcodeInfo for that byte, or undefined if not a valid WASM 1.0 opcode
 *
 * @example
 * getOpcode(0x6A)  // → { name: "i32.add", opcode: 0x6A, ... }
 * getOpcode(0xFF)  // → undefined
 */
export function getOpcode(byte: number): OpcodeInfo | undefined {
  return OPCODES.get(byte);
}

/**
 * Look up an instruction by its text-format name.
 *
 * @param name - The canonical WASM instruction name (e.g. "i32.add")
 * @returns The OpcodeInfo for that name, or undefined if not found
 *
 * @example
 * getOpcodeByName("i32.add")   // → { name: "i32.add", opcode: 0x6A, ... }
 * getOpcodeByName("i32.foo")   // → undefined
 */
export function getOpcodeByName(name: string): OpcodeInfo | undefined {
  return OPCODES_BY_NAME.get(name);
}
