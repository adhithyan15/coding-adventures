# frozen_string_literal: true

# =============================================================================
# wasm_opcodes.rb — Complete WASM 1.0 Opcode Table
# =============================================================================
#
# WebAssembly (WASM) is a binary instruction format designed as a portable
# compilation target for high-level languages like C, C++, and Rust.
# Each instruction in a WASM module is represented as a single byte (the
# opcode), followed by zero or more "immediate" operands encoded directly in
# the byte stream.
#
# This module provides a complete lookup table of all 172 WASM 1.0
# instructions, with metadata about each: name, byte value, category,
# immediate operand types, and stack effects.
#
# =============================================================================
# THE WASM STACK MACHINE
# =============================================================================
#
# WASM is a stack machine. Instructions operate on an implicit operand stack:
#
#   Before: [a, b]    i32.add    After: [a+b]
#
#   stack_pop  = number of values consumed from the stack
#   stack_push = number of values produced onto the stack
#
# For example:
#   i32.const: pops 0, pushes 1  (places a constant on the stack)
#   i32.add:   pops 2, pushes 1  (a + b → result)
#   i32.store: pops 2, pushes 0  (address + value → side effect)
#
# Control flow instructions (block, loop, if, call) have variable stack
# effects that depend on their type annotation or function signature.
# We record 0/0 for those as placeholders.
#
# =============================================================================
# IMMEDIATE OPERANDS
# =============================================================================
#
# After the opcode byte, some instructions encode additional data inline:
#
#   "i32"          — 32-bit integer, LEB128-encoded (variable width)
#   "i64"          — 64-bit integer, LEB128-encoded
#   "f32"          — 32-bit IEEE 754 float, 4 bytes little-endian
#   "f64"          — 64-bit IEEE 754 float, 8 bytes little-endian
#   "blocktype"    — result type of a structured block (-0x40 for void, or a
#                    valtype byte: 0x7F=i32, 0x7E=i64, 0x7D=f32, 0x7C=f64)
#   "labelidx"     — branch target, LEB128-encoded label stack depth
#   "vec_labelidx" — br_table: count followed by N label indices
#   "funcidx"      — index into the function table, LEB128-encoded
#   "typeidx"      — index into the type section
#   "tableidx"     — index into the table section (always 0 in WASM 1.0)
#   "localidx"     — index into the local variable list
#   "globalidx"    — index into the global variable list
#   "memarg"       — memory argument: { align: u32, offset: u32 }, both LEB128
#                    align is log2 of the expected alignment hint
#                    offset is a static byte offset added to the runtime address
#   "memidx"       — index into the memory section (always 0 in WASM 1.0)
#
# =============================================================================
# CATEGORIES
# =============================================================================
#
#   "control"      — structured control flow (block, loop, if, br, call, ...)
#   "parametric"   — stack manipulation (drop, select)
#   "variable"     — local/global variable access
#   "memory"       — heap loads, stores, and memory management
#   "numeric_i32"  — 32-bit integer arithmetic, comparisons, bitwise ops
#   "numeric_i64"  — 64-bit integer arithmetic, comparisons, bitwise ops
#   "numeric_f32"  — 32-bit floating-point arithmetic and comparisons
#   "numeric_f64"  — 64-bit floating-point arithmetic and comparisons
#   "conversion"   — type conversions between numeric types
#
# =============================================================================
# COMPLETE OPCODE TABLE (all 172 WASM 1.0 instructions)
# =============================================================================
#
# Control flow:
#   0x00  unreachable             — trap unconditionally
#   0x01  nop                     — no operation
#   0x02  block [blocktype]       — begin a structured block
#   0x03  loop  [blocktype]       — begin a loop (br targets loop top)
#   0x04  if    [blocktype]       — conditional: pops 1 i32 condition
#   0x05  else                    — else arm of if block
#   0x0B  end                     — end block/loop/if/function
#   0x0C  br    [labelidx]        — unconditional branch
#   0x0D  br_if [labelidx]        — conditional branch: pops 1 condition
#   0x0E  br_table [vec_labelidx] — indirect branch table: pops 1 index
#   0x0F  return                  — return from current function
#   0x10  call   [funcidx]        — direct function call
#   0x11  call_indirect [typeidx, tableidx] — call through function table
#
# Parametric:
#   0x1A  drop   — discard top-of-stack value (any type)
#   0x1B  select — pops (cond:i32, b:T, a:T), pushes a if cond≠0 else b
#
# Variables:
#   0x20  local.get  [localidx]  — push local onto stack
#   0x21  local.set  [localidx]  — pop and write to local
#   0x22  local.tee  [localidx]  — write to local without consuming (peek+store)
#   0x23  global.get [globalidx] — push global onto stack
#   0x24  global.set [globalidx] — pop and write to global
#
# Memory loads (pop i32 address, push loaded value; all take [memarg]):
#   0x28  i32.load, 0x29 i64.load, 0x2A f32.load, 0x2B f64.load
#   0x2C  i32.load8_s, 0x2D i32.load8_u, 0x2E i32.load16_s, 0x2F i32.load16_u
#   0x30  i64.load8_s, 0x31 i64.load8_u, 0x32 i64.load16_s, 0x33 i64.load16_u
#   0x34  i64.load32_s, 0x35 i64.load32_u
#
# Memory stores (pop i32 address + value; all take [memarg]):
#   0x36  i32.store, 0x37 i64.store, 0x38 f32.store, 0x39 f64.store
#   0x3A  i32.store8, 0x3B i32.store16
#   0x3C  i64.store8, 0x3D i64.store16, 0x3E i64.store32
#
# Memory management:
#   0x3F  memory.size [memidx] — push page count
#   0x40  memory.grow [memidx] — grow memory; pops delta, pushes old size
#

module CodingAdventures
  module WasmOpcodes
    # OpcodeInfo — metadata record for a single WASM instruction.
    #
    # Fields:
    #   name       — canonical text-format name, e.g. "i32.add"
    #   opcode     — single-byte encoding, e.g. 0x6A (= 106 decimal)
    #   category   — instruction group, e.g. "numeric_i32"
    #   immediates — array of immediate operand type strings (may be empty)
    #   stack_pop  — number of stack values consumed
    #   stack_push — number of stack values produced
    OpcodeInfo = Struct.new(:name, :opcode, :category, :immediates, :stack_pop, :stack_push, keyword_init: true)

    # -------------------------------------------------------------------------
    # Raw opcode table
    # -------------------------------------------------------------------------
    # Each entry: [opcode, name, category, immediates, stack_pop, stack_push]
    # Grouped by category for readability, in WASM spec order.
    # -------------------------------------------------------------------------

    RAW_OPCODES = [
      # ── Control flow ─────────────────────────────────────────────────────────
      # WASM uses structured control flow: no arbitrary jumps.
      # Blocks form a label stack. br/br_if jump to a label by depth index.
      #
      #   block [] — "br 0" exits the block (forward jump)
      #   loop  [] — "br 0" goes back to the top (backward jump)
      #   if    [] — pops an i32 condition; if nonzero, executes the if arm
      #
      # Stack effects for block/loop/if/call are 0/0 because the true effect
      # depends on the blocktype annotation or function type signature.
      [0x00, "unreachable",   "control", [],                       0, 0],
      [0x01, "nop",           "control", [],                       0, 0],
      [0x02, "block",         "control", ["blocktype"],            0, 0],
      [0x03, "loop",          "control", ["blocktype"],            0, 0],
      [0x04, "if",            "control", ["blocktype"],            1, 0],
      [0x05, "else",          "control", [],                       0, 0],
      [0x0B, "end",           "control", [],                       0, 0],
      [0x0C, "br",            "control", ["labelidx"],             0, 0],
      [0x0D, "br_if",         "control", ["labelidx"],             1, 0],
      [0x0E, "br_table",      "control", ["vec_labelidx"],         1, 0],
      [0x0F, "return",        "control", [],                       0, 0],
      [0x10, "call",          "control", ["funcidx"],              0, 0],
      [0x11, "call_indirect", "control", ["typeidx", "tableidx"],  1, 0],

      # ── Parametric ───────────────────────────────────────────────────────────
      # These instructions are polymorphic — they work on values of any type.
      #
      #   drop   — discard TOS (top of stack), consuming but not using it
      #   select — like a ternary: pops (c:i32, b:T, a:T), pushes a if c≠0
      [0x1A, "drop",   "parametric", [], 1, 0],
      [0x1B, "select", "parametric", [], 3, 1],

      # ── Variable access ──────────────────────────────────────────────────────
      # WASM variables are indexed (not named). Locals 0..n-1 include
      # function parameters followed by declared locals.
      #
      #   local.tee — like local.set but also leaves the value on the stack.
      #               Think of it as "peek + store" instead of "pop + store".
      [0x20, "local.get",  "variable", ["localidx"],  0, 1],
      [0x21, "local.set",  "variable", ["localidx"],  1, 0],
      [0x22, "local.tee",  "variable", ["localidx"],  1, 1],
      [0x23, "global.get", "variable", ["globalidx"], 0, 1],
      [0x24, "global.set", "variable", ["globalidx"], 1, 0],

      # ── Memory loads ─────────────────────────────────────────────────────────
      # All loads: pop one i32 (base address), push one loaded value.
      # effective_address = memarg.offset + popped_address
      #
      # _s = sign-extend  (fill upper bits with the sign bit)
      # _u = zero-extend  (fill upper bits with 0)
      #
      # The memarg immediate is two LEB128 values:
      #   align  — log₂ of expected alignment (hint only, not enforced)
      #   offset — static byte offset added to the runtime address
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

      # ── Memory stores ────────────────────────────────────────────────────────
      # All stores: pop two values — base address (i32) and value to write.
      # Truncating stores (store8, store16, store32) discard the high bits.
      [0x36, "i32.store",   "memory", ["memarg"], 2, 0],
      [0x37, "i64.store",   "memory", ["memarg"], 2, 0],
      [0x38, "f32.store",   "memory", ["memarg"], 2, 0],
      [0x39, "f64.store",   "memory", ["memarg"], 2, 0],
      [0x3A, "i32.store8",  "memory", ["memarg"], 2, 0],
      [0x3B, "i32.store16", "memory", ["memarg"], 2, 0],
      [0x3C, "i64.store8",  "memory", ["memarg"], 2, 0],
      [0x3D, "i64.store16", "memory", ["memarg"], 2, 0],
      [0x3E, "i64.store32", "memory", ["memarg"], 2, 0],

      # ── Memory management ────────────────────────────────────────────────────
      # WASM memory is a contiguous byte array measured in 64 KiB pages.
      #   memory.size — returns current page count (no arguments)
      #   memory.grow — grows memory by N pages, returns old size or -1 on fail
      [0x3F, "memory.size", "memory", ["memidx"], 0, 1],
      [0x40, "memory.grow", "memory", ["memidx"], 1, 1],

      # ── i32 numeric ──────────────────────────────────────────────────────────
      # 32-bit integer arithmetic. WASM uses two's complement representation.
      #
      #   clz    — count leading zeros  (from MSB)
      #   ctz    — count trailing zeros (from LSB)
      #   popcnt — count set bits (population count / Hamming weight)
      #   eqz    — test if zero; produces i32 result: 1 if zero, 0 if nonzero
      #   eq/ne/lt/gt/le/ge — comparisons; produce i32 result 0 or 1
      #   _s     — signed interpretation (two's complement)
      #   _u     — unsigned interpretation
      [0x41, "i32.const",  "numeric_i32", ["i32"], 0, 1],
      [0x45, "i32.eqz",    "numeric_i32", [],      1, 1],
      [0x46, "i32.eq",     "numeric_i32", [],      2, 1],
      [0x47, "i32.ne",     "numeric_i32", [],      2, 1],
      [0x48, "i32.lt_s",   "numeric_i32", [],      2, 1],
      [0x49, "i32.lt_u",   "numeric_i32", [],      2, 1],
      [0x4A, "i32.gt_s",   "numeric_i32", [],      2, 1],
      [0x4B, "i32.gt_u",   "numeric_i32", [],      2, 1],
      [0x4C, "i32.le_s",   "numeric_i32", [],      2, 1],
      [0x4D, "i32.le_u",   "numeric_i32", [],      2, 1],
      [0x4E, "i32.ge_s",   "numeric_i32", [],      2, 1],
      [0x4F, "i32.ge_u",   "numeric_i32", [],      2, 1],
      [0x67, "i32.clz",    "numeric_i32", [],      1, 1],
      [0x68, "i32.ctz",    "numeric_i32", [],      1, 1],
      [0x69, "i32.popcnt", "numeric_i32", [],      1, 1],
      [0x6A, "i32.add",    "numeric_i32", [],      2, 1],
      [0x6B, "i32.sub",    "numeric_i32", [],      2, 1],
      [0x6C, "i32.mul",    "numeric_i32", [],      2, 1],
      [0x6D, "i32.div_s",  "numeric_i32", [],      2, 1],
      [0x6E, "i32.div_u",  "numeric_i32", [],      2, 1],
      [0x6F, "i32.rem_s",  "numeric_i32", [],      2, 1],
      [0x70, "i32.rem_u",  "numeric_i32", [],      2, 1],
      [0x71, "i32.and",    "numeric_i32", [],      2, 1],
      [0x72, "i32.or",     "numeric_i32", [],      2, 1],
      [0x73, "i32.xor",    "numeric_i32", [],      2, 1],
      [0x74, "i32.shl",    "numeric_i32", [],      2, 1],
      [0x75, "i32.shr_s",  "numeric_i32", [],      2, 1],
      [0x76, "i32.shr_u",  "numeric_i32", [],      2, 1],
      [0x77, "i32.rotl",   "numeric_i32", [],      2, 1],
      [0x78, "i32.rotr",   "numeric_i32", [],      2, 1],

      # ── i64 numeric ──────────────────────────────────────────────────────────
      # 64-bit integer arithmetic. Same operations as i32 but 64-bit wide.
      # Note: comparison results (eqz, eq, ne, lt, ...) are still i32 (0 or 1).
      [0x42, "i64.const",  "numeric_i64", ["i64"], 0, 1],
      [0x50, "i64.eqz",    "numeric_i64", [],      1, 1],
      [0x51, "i64.eq",     "numeric_i64", [],      2, 1],
      [0x52, "i64.ne",     "numeric_i64", [],      2, 1],
      [0x53, "i64.lt_s",   "numeric_i64", [],      2, 1],
      [0x54, "i64.lt_u",   "numeric_i64", [],      2, 1],
      [0x55, "i64.gt_s",   "numeric_i64", [],      2, 1],
      [0x56, "i64.gt_u",   "numeric_i64", [],      2, 1],
      [0x57, "i64.le_s",   "numeric_i64", [],      2, 1],
      [0x58, "i64.le_u",   "numeric_i64", [],      2, 1],
      [0x59, "i64.ge_s",   "numeric_i64", [],      2, 1],
      [0x5A, "i64.ge_u",   "numeric_i64", [],      2, 1],
      [0x79, "i64.clz",    "numeric_i64", [],      1, 1],
      [0x7A, "i64.ctz",    "numeric_i64", [],      1, 1],
      [0x7B, "i64.popcnt", "numeric_i64", [],      1, 1],
      [0x7C, "i64.add",    "numeric_i64", [],      2, 1],
      [0x7D, "i64.sub",    "numeric_i64", [],      2, 1],
      [0x7E, "i64.mul",    "numeric_i64", [],      2, 1],
      [0x7F, "i64.div_s",  "numeric_i64", [],      2, 1],
      [0x80, "i64.div_u",  "numeric_i64", [],      2, 1],
      [0x81, "i64.rem_s",  "numeric_i64", [],      2, 1],
      [0x82, "i64.rem_u",  "numeric_i64", [],      2, 1],
      [0x83, "i64.and",    "numeric_i64", [],      2, 1],
      [0x84, "i64.or",     "numeric_i64", [],      2, 1],
      [0x85, "i64.xor",    "numeric_i64", [],      2, 1],
      [0x86, "i64.shl",    "numeric_i64", [],      2, 1],
      [0x87, "i64.shr_s",  "numeric_i64", [],      2, 1],
      [0x88, "i64.shr_u",  "numeric_i64", [],      2, 1],
      [0x89, "i64.rotl",   "numeric_i64", [],      2, 1],
      [0x8A, "i64.rotr",   "numeric_i64", [],      2, 1],

      # ── f32 numeric ──────────────────────────────────────────────────────────
      # 32-bit IEEE 754 single-precision floating point.
      #
      #   abs/neg       — absolute value / negate (flip sign bit)
      #   ceil/floor    — round toward +∞ / −∞
      #   trunc/nearest — round toward zero / round to nearest even
      #   sqrt          — square root
      #   min/max       — IEEE min/max (NaN propagating)
      #   copysign      — copy sign: copysign(a, b) = |a| with sign of b
      #
      # Comparisons (eq, ne, lt, gt, le, ge) produce i32 (0 or 1), not f32.
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

      # ── f64 numeric ──────────────────────────────────────────────────────────
      # 64-bit IEEE 754 double-precision floating point.
      # Same operations as f32 but with greater range and precision.
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

      # ── Conversions ──────────────────────────────────────────────────────────
      # All conversions: pop 1, push 1. No immediates.
      #
      #   wrap_i64         — truncate i64 to i32 (drop high 32 bits)
      #   extend_i32_s/u   — widen i32 to i64 (sign- or zero-extend)
      #   trunc_fNN_s/u    — truncate float to integer (traps on NaN/overflow)
      #   convert_iNN_s/u  — convert integer to float (exact or rounded)
      #   demote_f64       — f64 → f32 (narrowing; may lose precision)
      #   promote_f32      — f32 → f64 (widening; always exact)
      #   reinterpret_*    — reinterpret bit pattern without numeric conversion
      #
      # The reinterpret instructions are useful for bit-level manipulation.
      # For example, to inspect the sign bit of an f32:
      #   f32.reinterpret_i32 → i32.const 31 → i32.shr_u → (sign bit in bit 0)
      [0xA7, "i32.wrap_i64",        "conversion", [], 1, 1],
      [0xA8, "i32.trunc_f32_s",     "conversion", [], 1, 1],
      [0xA9, "i32.trunc_f32_u",     "conversion", [], 1, 1],
      [0xAA, "i32.trunc_f64_s",     "conversion", [], 1, 1],
      [0xAB, "i32.trunc_f64_u",     "conversion", [], 1, 1],
      [0xAC, "i64.extend_i32_s",    "conversion", [], 1, 1],
      [0xAD, "i64.extend_i32_u",    "conversion", [], 1, 1],
      [0xAE, "i64.trunc_f32_s",     "conversion", [], 1, 1],
      [0xAF, "i64.trunc_f32_u",     "conversion", [], 1, 1],
      [0xB0, "i64.trunc_f64_s",     "conversion", [], 1, 1],
      [0xB1, "i64.trunc_f64_u",     "conversion", [], 1, 1],
      [0xB2, "f32.convert_i32_s",   "conversion", [], 1, 1],
      [0xB3, "f32.convert_i32_u",   "conversion", [], 1, 1],
      [0xB4, "f32.convert_i64_s",   "conversion", [], 1, 1],
      [0xB5, "f32.convert_i64_u",   "conversion", [], 1, 1],
      [0xB6, "f32.demote_f64",      "conversion", [], 1, 1],
      [0xB7, "f64.convert_i32_s",   "conversion", [], 1, 1],
      [0xB8, "f64.convert_i32_u",   "conversion", [], 1, 1],
      [0xB9, "f64.convert_i64_s",   "conversion", [], 1, 1],
      [0xBA, "f64.convert_i64_u",   "conversion", [], 1, 1],
      [0xBB, "f64.promote_f32",     "conversion", [], 1, 1],
      [0xBC, "i32.reinterpret_f32", "conversion", [], 1, 1],
      [0xBD, "i64.reinterpret_f64", "conversion", [], 1, 1],
      [0xBE, "f32.reinterpret_i32", "conversion", [], 1, 1],
      [0xBF, "f64.reinterpret_i64", "conversion", [], 1, 1],
    ].freeze

    # -------------------------------------------------------------------------
    # Build the lookup tables from the raw data
    # -------------------------------------------------------------------------

    # OPCODES — primary lookup by opcode byte.
    #
    # Hash[Integer, OpcodeInfo] built once at load time.
    # Keys are the single-byte opcode values (0x00–0xBF).
    #
    # @example
    #   info = CodingAdventures::WasmOpcodes::OPCODES[0x6A]  # => i32.add
    OPCODES = RAW_OPCODES.each_with_object({}) do |(opcode, name, category, immediates, stack_pop, stack_push), h|
      h[opcode] = OpcodeInfo.new(
        name: name,
        opcode: opcode,
        category: category,
        immediates: immediates.freeze,
        stack_pop: stack_pop,
        stack_push: stack_push
      ).freeze
    end.freeze

    # OPCODES_BY_NAME — secondary lookup by text-format name.
    #
    # Hash[String, OpcodeInfo] mirroring OPCODES but keyed by name.
    # Both hashes always have the same size (one entry per instruction).
    #
    # @example
    #   info = CodingAdventures::WasmOpcodes::OPCODES_BY_NAME["i32.add"]
    OPCODES_BY_NAME = OPCODES.values.each_with_object({}) do |info, h|
      h[info.name] = info
    end.freeze

    # -------------------------------------------------------------------------
    # Public API
    # -------------------------------------------------------------------------

    # Look up an instruction by its opcode byte.
    #
    # @param byte [Integer] the opcode byte value (0x00–0xBF for WASM 1.0)
    # @return [OpcodeInfo, nil] the OpcodeInfo, or nil if not a valid WASM 1.0 opcode
    #
    # @example
    #   get_opcode(0x6A)   # => #<OpcodeInfo name="i32.add" ...>
    #   get_opcode(0xFF)   # => nil
    def self.get_opcode(byte)
      OPCODES[byte]
    end

    # Look up an instruction by its text-format name.
    #
    # @param name [String] the canonical WASM instruction name (e.g. "i32.add")
    # @return [OpcodeInfo, nil] the OpcodeInfo, or nil if not found
    #
    # @example
    #   get_opcode_by_name("i32.add")   # => #<OpcodeInfo name="i32.add" ...>
    #   get_opcode_by_name("i32.foo")   # => nil
    def self.get_opcode_by_name(name)
      OPCODES_BY_NAME[name]
    end
  end
end
