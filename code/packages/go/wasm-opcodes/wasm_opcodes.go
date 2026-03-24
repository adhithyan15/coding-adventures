// Package wasmopcodes provides a complete lookup table for all WASM 1.0
// instructions, with metadata for each opcode: its byte value, mnemonic name,
// category, immediate operands, and stack effect (values consumed and produced).
//
// This package is part of the coding-adventures monorepo, a ground-up
// implementation of the computing stack from transistors to operating systems.
//
// # What is a WASM Opcode?
//
// A WebAssembly module's code section contains sequences of *instructions*.
// Each instruction starts with a 1-byte opcode (0x00–0xFF) that tells the
// WASM runtime what operation to perform. Some instructions are followed by
// *immediates* — additional encoded values (a constant integer, a memory
// offset, a label index) that parameterize the instruction.
//
// WASM is a *stack machine*: instructions communicate via an implicit operand
// stack. For example, i32.add does not name registers; it pops two i32 values
// and pushes their sum. The "stack effect" of an instruction is simply:
// how many values does it pop (StackPop), and how many does it push (StackPush)?
//
// # Why Structured Control Flow?
//
// Traditional bytecode formats (JVM, x86) use arbitrary jumps — a branch
// instruction gives a raw byte offset to jump to anywhere. WASM deliberately
// forbids arbitrary jumps. Instead, control flow is *structured*:
//
//   - block / loop / if   push a control frame (a nesting level)
//   - br / br_if          branch by nesting *depth* (0 = innermost)
//   - end                 close the innermost structured block
//
// Advantages of structured control flow:
//  1. Validation is a single linear pass — no control-flow graph needed.
//  2. JIT compilation is straightforward — structured blocks map directly to
//     native conditional branches and loops.
//  3. Security: impossible to jump into the middle of an instruction.
//  4. Streaming: WASM can be compiled while still downloading (top-to-bottom).
//
// # What is a Memarg?
//
// Memory load and store instructions need two extra pieces of information:
//
//   - align:  Log2 of expected alignment (e.g., align=2 means 4-byte aligned).
//             The runtime may generate faster code with this hint, but must
//             still handle misaligned accesses correctly.
//   - offset: A constant byte offset added to the runtime address on the stack.
//             This lets compilers encode struct field access as a single
//             instruction: load base pointer, add constant field offset.
//
// Both values are encoded as LEB128 integers immediately after the opcode byte.
// Together they are called a "memarg".
//
// Example:
//
//	0x28  0x02  0x04
//	^^^^  ^^^^  ^^^^
//	i32.load  align=2 (4-byte aligned)  offset=4
//
// This reads 4 bytes from memory at address (stack_top + 4).
//
// # Opcode Categories
//
//	"control"      structured control flow: block, loop, if, br, call, ...
//	"parametric"   type-agnostic stack manipulation: drop, select
//	"variable"     local and global variable access
//	"memory"       loads, stores, memory.size, memory.grow
//	"numeric_i32"  32-bit integer arithmetic, comparisons, bitwise ops
//	"numeric_i64"  64-bit integer arithmetic, comparisons, bitwise ops
//	"numeric_f32"  32-bit floating-point arithmetic
//	"numeric_f64"  64-bit floating-point arithmetic
//	"conversion"   type conversions: wrap, trunc, extend, reinterpret
//
// # Complete Opcode Table (WASM 1.0 — 172 instructions)
//
// Control:
//
//	0x00  unreachable        ()                    pop=0 push=0
//	0x01  nop                ()                    pop=0 push=0
//	0x02  block              ("blocktype")         pop=0 push=0
//	0x03  loop               ("blocktype")         pop=0 push=0
//	0x04  if                 ("blocktype")         pop=1 push=0
//	0x05  else               ()                    pop=0 push=0
//	0x0B  end                ()                    pop=0 push=0
//	0x0C  br                 ("labelidx")          pop=0 push=0
//	0x0D  br_if              ("labelidx")          pop=1 push=0
//	0x0E  br_table           ("vec_labelidx")      pop=1 push=0
//	0x0F  return             ()                    pop=0 push=0
//	0x10  call               ("funcidx")           pop=0 push=0
//	0x11  call_indirect      ("typeidx","tableidx") pop=1 push=0
//
// Parametric:
//
//	0x1A  drop               ()                    pop=1 push=0
//	0x1B  select             ()                    pop=3 push=1
//
// Variable:
//
//	0x20  local.get          ("localidx")          pop=0 push=1
//	0x21  local.set          ("localidx")          pop=1 push=0
//	0x22  local.tee          ("localidx")          pop=1 push=1
//	0x23  global.get         ("globalidx")         pop=0 push=1
//	0x24  global.set         ("globalidx")         pop=1 push=0
//
// Memory loads (all have ["memarg"] immediates, pop=1, push=1):
//
//	0x28 i32.load   0x29 i64.load   0x2A f32.load   0x2B f64.load
//	0x2C i32.load8_s  0x2D i32.load8_u  0x2E i32.load16_s  0x2F i32.load16_u
//	0x30 i64.load8_s  0x31 i64.load8_u  0x32 i64.load16_s  0x33 i64.load16_u
//	0x34 i64.load32_s  0x35 i64.load32_u
//
// Memory stores (all have ["memarg"] immediates, pop=2, push=0):
//
//	0x36 i32.store  0x37 i64.store  0x38 f32.store  0x39 f64.store
//	0x3A i32.store8  0x3B i32.store16  0x3C i64.store8  0x3D i64.store16
//	0x3E i64.store32
//
// Memory management:
//
//	0x3F  memory.size  ("memidx")  pop=0 push=1
//	0x40  memory.grow  ("memidx")  pop=1 push=1
//
// i32 numeric (0x41 const, 0x45-0x4F comparisons, 0x67-0x78 arithmetic):
//
//	0x41 i32.const("i32") pop=0 push=1
//	0x45 i32.eqz pop=1    0x46-0x4F i32.eq/ne/lt_s/lt_u/gt_s/gt_u/le_s/le_u/ge_s/ge_u pop=2
//	0x67-0x69 i32.clz/ctz/popcnt pop=1
//	0x6A-0x78 i32.add/sub/mul/div_s/div_u/rem_s/rem_u/and/or/xor/shl/shr_s/shr_u/rotl/rotr pop=2
//
// i64 numeric (mirrors i32): 0x42 const, 0x50-0x5A, 0x79-0x8A
// f32 numeric: 0x43 const, 0x5B-0x60 compare, 0x8B-0x98 arithmetic
// f64 numeric: 0x44 const, 0x61-0x66 compare, 0x99-0xA6 arithmetic
//
// Conversions (all pop=1, push=1):
//
//	0xA7-0xBF: wrap, trunc, extend, convert, demote, promote, reinterpret
package wasmopcodes

// ---------------------------------------------------------------------------
// OPCODEINFO — METADATA FOR A SINGLE WASM INSTRUCTION
//
// Each field documents one aspect of the instruction's specification:
//
//   Name       — the mnemonic from the WASM text format (.wat files)
//                Examples: "i32.add", "memory.grow", "br_if"
//
//   Opcode     — the 1-byte value in the binary encoding
//                Example: 0x6A for i32.add
//
//   Category   — groups related instructions together:
//                "control", "parametric", "variable", "memory",
//                "numeric_i32", "numeric_i64", "numeric_f32",
//                "numeric_f64", "conversion"
//
//   Immediates — slice of strings naming the encoded operands that follow
//                the opcode byte. These are NOT the operand stack values
//                (those are captured by StackPop/StackPush). Immediates are
//                explicit bytes embedded in the instruction stream.
//                Known immediate tokens:
//                  []           — no immediates (e.g., i32.add)
//                  ["i32"]      — LEB128 signed 32-bit constant (i32.const)
//                  ["i64"]      — LEB128 signed 64-bit constant (i64.const)
//                  ["f32"]      — 4-byte IEEE 754 float (f32.const)
//                  ["f64"]      — 8-byte IEEE 754 double (f64.const)
//                  ["memarg"]   — align + offset (all loads and stores)
//                  ["labelidx"] — branch target depth (br, br_if)
//                  ["vec_labelidx"] — vector of depths + default (br_table)
//                  ["blocktype"] — block result type (block, loop, if)
//                  ["funcidx"]  — function index (call)
//                  ["localidx"] — local variable index (local.get/set/tee)
//                  ["globalidx"] — global variable index (global.get/set)
//                  ["memidx"]   — memory index, always 0 in WASM 1.0
//                  ["typeidx", "tableidx"] — two indices (call_indirect)
//
//   StackPop   — number of values consumed from the operand stack
//   StackPush  — number of values produced onto the operand stack
//
// Stack effect analogy: think of a cafeteria plate dispenser. StackPop = how
// many plates you take off the top. StackPush = how many clean plates you
// place back on top. Net effect = StackPush - StackPop.
//
// Note: for call and call_indirect, the actual arity depends on the called
// function's type and is not a fixed number. We record pop=0/push=0 for call
// (the argument pops are type-dependent) and pop=1 for call_indirect (the
// table-index pop; arguments are again type-dependent).
// ---------------------------------------------------------------------------

// OpcodeInfo holds the metadata for a single WASM 1.0 instruction.
//
// Example:
//
//	info := OpcodeInfo{
//	    Name:       "i32.add",
//	    Opcode:     0x6A,
//	    Category:   "numeric_i32",
//	    Immediates: nil,   // no immediates
//	    StackPop:   2,
//	    StackPush:  1,
//	}
type OpcodeInfo struct {
	Name       string   // WASM text-format mnemonic (e.g., "i32.add")
	Opcode     byte     // binary encoding byte value (e.g., 0x6A)
	Category   string   // instruction group (e.g., "numeric_i32")
	Immediates []string // names of immediate operands encoded after the opcode
	StackPop   int      // values consumed from the operand stack
	StackPush  int      // values pushed onto the operand stack
}

// ---------------------------------------------------------------------------
// RAW TABLE — THE MASTER LIST OF ALL 172 WASM 1.0 INSTRUCTIONS
//
// Defined as a slice of rawEntry structs for clarity. The table is processed
// once at package init() time to build two lookup maps:
//   Opcodes       — keyed by opcode byte value
//   OpcodesByName — keyed by mnemonic string
//
// Each row in the table becomes one OpcodeInfo entry in both maps.
// ---------------------------------------------------------------------------

type rawEntry struct {
	name       string
	opcode     byte
	category   string
	immediates []string
	stackPop   int
	stackPush  int
}

//nolint:gochecknoglobals
var rawTable = []rawEntry{
	// -----------------------------------------------------------------------
	// CONTROL — structured control flow
	//
	// WASM's structured control flow is explained in the package docstring.
	// Key points:
	//   - "unreachable" traps immediately if ever executed. It marks dead code
	//     — the validator knows code after an unconditional branch is unreachable.
	//   - "block" exits forward (br 0 = jump to the END of the block).
	//   - "loop" exits backward (br 0 = jump to the START of the loop — looping).
	//   - "br N" branches to the Nth enclosing label (0 = innermost).
	//   - "br_table" is a switch: it pops an i32 index and jumps to one of
	//     N targets, or the default target if the index is out of range.
	//   - "call" pop/push is 0 here; actual arity comes from the function type.
	//   - "call_indirect" pops the i32 table index (1 pop); arguments are
	//     also popped but their count comes from the type immediate.
	// -----------------------------------------------------------------------
	{"unreachable",   0x00, "control",    nil,                       0, 0},
	{"nop",           0x01, "control",    nil,                       0, 0},
	{"block",         0x02, "control",    []string{"blocktype"},     0, 0},
	{"loop",          0x03, "control",    []string{"blocktype"},     0, 0},
	{"if",            0x04, "control",    []string{"blocktype"},     1, 0},
	{"else",          0x05, "control",    nil,                       0, 0},
	{"end",           0x0B, "control",    nil,                       0, 0},
	{"br",            0x0C, "control",    []string{"labelidx"},      0, 0},
	{"br_if",         0x0D, "control",    []string{"labelidx"},      1, 0},
	{"br_table",      0x0E, "control",    []string{"vec_labelidx"},  1, 0},
	{"return",        0x0F, "control",    nil,                       0, 0},
	{"call",          0x10, "control",    []string{"funcidx"},       0, 0},
	{"call_indirect", 0x11, "control",    []string{"typeidx", "tableidx"}, 1, 0},

	// -----------------------------------------------------------------------
	// PARAMETRIC — type-agnostic stack manipulation
	//
	// "drop" discards the top-of-stack value regardless of its type.
	// "select" is WASM's ternary (?:). Stack before: [val_true, val_false, cond]
	//   where cond is on top. If cond != 0, result = val_true; else val_false.
	//   NOTE: both values are already evaluated (no short-circuit). This differs
	//   from if/else where only one branch executes.
	// -----------------------------------------------------------------------
	{"drop",   0x1A, "parametric", nil, 1, 0},
	{"select", 0x1B, "parametric", nil, 3, 1},

	// -----------------------------------------------------------------------
	// VARIABLE — local and global variable access
	//
	// Locals 0..n_params-1 are the function's parameters.
	// Locals n_params..n_params+n_locals-1 are declared local variables.
	//
	// "local.tee" = "local.set" but also keeps the value on the stack.
	// Name comes from a T-junction: value flows to the slot AND stays on stack.
	// Equivalent to: local.set N; local.get N
	//
	// "global.set" only works on mutable globals; the validator rejects it
	// for immutable globals at parse time.
	// -----------------------------------------------------------------------
	{"local.get",  0x20, "variable", []string{"localidx"},  0, 1},
	{"local.set",  0x21, "variable", []string{"localidx"},  1, 0},
	{"local.tee",  0x22, "variable", []string{"localidx"},  1, 1},
	{"global.get", 0x23, "variable", []string{"globalidx"}, 0, 1},
	{"global.set", 0x24, "variable", []string{"globalidx"}, 1, 0},

	// -----------------------------------------------------------------------
	// MEMORY LOADS — read from linear memory
	//
	// All loads:
	//   1. Pop one i32 address from the stack.
	//   2. Add the memarg offset (constant immediate) to get effective address.
	//   3. Read N bytes from memory at effective address.
	//   4. Optionally sign-extend (_s) or zero-extend (_u) to full width.
	//   5. Push the result value.
	//
	// "_s" = sign-extend (fill high bits with the sign bit of the loaded value).
	//   byte 0xFF with _s becomes -1 (0xFFFF_FFFF as i32)
	// "_u" = zero-extend (fill high bits with 0).
	//   byte 0xFF with _u becomes 255 (0x0000_00FF as i32)
	// -----------------------------------------------------------------------
	{"i32.load",    0x28, "memory", []string{"memarg"}, 1, 1},
	{"i64.load",    0x29, "memory", []string{"memarg"}, 1, 1},
	{"f32.load",    0x2A, "memory", []string{"memarg"}, 1, 1},
	{"f64.load",    0x2B, "memory", []string{"memarg"}, 1, 1},
	{"i32.load8_s", 0x2C, "memory", []string{"memarg"}, 1, 1},
	{"i32.load8_u", 0x2D, "memory", []string{"memarg"}, 1, 1},
	{"i32.load16_s",0x2E, "memory", []string{"memarg"}, 1, 1},
	{"i32.load16_u",0x2F, "memory", []string{"memarg"}, 1, 1},
	{"i64.load8_s", 0x30, "memory", []string{"memarg"}, 1, 1},
	{"i64.load8_u", 0x31, "memory", []string{"memarg"}, 1, 1},
	{"i64.load16_s",0x32, "memory", []string{"memarg"}, 1, 1},
	{"i64.load16_u",0x33, "memory", []string{"memarg"}, 1, 1},
	{"i64.load32_s",0x34, "memory", []string{"memarg"}, 1, 1},
	{"i64.load32_u",0x35, "memory", []string{"memarg"}, 1, 1},

	// -----------------------------------------------------------------------
	// MEMORY STORES — write to linear memory
	//
	// Store instructions pop TWO values:
	//   1. The i32 address (top of stack — pushed last)
	//   2. The value to store (second from top — pushed first)
	//
	// Stack layout before store:
	//   [ value | address ]   ← address on top
	//
	// Truncating stores write only the low N bits; high bits are discarded.
	// -----------------------------------------------------------------------
	{"i32.store",   0x36, "memory", []string{"memarg"}, 2, 0},
	{"i64.store",   0x37, "memory", []string{"memarg"}, 2, 0},
	{"f32.store",   0x38, "memory", []string{"memarg"}, 2, 0},
	{"f64.store",   0x39, "memory", []string{"memarg"}, 2, 0},
	{"i32.store8",  0x3A, "memory", []string{"memarg"}, 2, 0},
	{"i32.store16", 0x3B, "memory", []string{"memarg"}, 2, 0},
	{"i64.store8",  0x3C, "memory", []string{"memarg"}, 2, 0},
	{"i64.store16", 0x3D, "memory", []string{"memarg"}, 2, 0},
	{"i64.store32", 0x3E, "memory", []string{"memarg"}, 2, 0},

	// -----------------------------------------------------------------------
	// MEMORY MANAGEMENT
	//
	// "memory.size" pushes the current size of the memory in 64-KiB pages.
	// "memory.grow" pops the number of pages to add, pushes the previous size
	//   (or -1 if growth failed due to OOM or exceeding the declared maximum).
	//
	// The "memidx" immediate is always 0 in WASM 1.0 (one memory per module).
	// It is included in the binary encoding for forward-compatibility with the
	// multi-memory proposal.
	//
	// Memory size calculation:
	//   byte_size = page_count * 65536   (1 page = 64 KiB = 65536 bytes)
	//   max WASM 1.0 memory = 65536 pages * 65536 bytes = 4 GiB
	// -----------------------------------------------------------------------
	{"memory.size", 0x3F, "memory", []string{"memidx"}, 0, 1},
	{"memory.grow", 0x40, "memory", []string{"memidx"}, 1, 1},

	// -----------------------------------------------------------------------
	// i32 NUMERIC — 32-bit integer operations
	//
	// "i32.const" pushes a compile-time i32 constant. Encoded as signed LEB128.
	//
	// Comparisons push 1 (true) or 0 (false) as i32. WASM has no boolean type.
	//
	// "eqz" tests equality with zero. Unlike eq/ne it pops only one operand.
	// Useful for "while (x)" loops and negation without a separate zero constant.
	//
	// "clz" = count leading zeros. "ctz" = count trailing zeros.
	// "popcnt" = population count (number of 1-bits in the value).
	//
	// "_s" = signed interpretation, "_u" = unsigned.
	// For add/sub/mul the bit pattern is sign-independent, so there's only
	// one i32.add (no signed/unsigned split for these operations).
	// -----------------------------------------------------------------------
	{"i32.const",  0x41, "numeric_i32", []string{"i32"}, 0, 1},
	{"i32.eqz",    0x45, "numeric_i32", nil,             1, 1},
	{"i32.eq",     0x46, "numeric_i32", nil,             2, 1},
	{"i32.ne",     0x47, "numeric_i32", nil,             2, 1},
	{"i32.lt_s",   0x48, "numeric_i32", nil,             2, 1},
	{"i32.lt_u",   0x49, "numeric_i32", nil,             2, 1},
	{"i32.gt_s",   0x4A, "numeric_i32", nil,             2, 1},
	{"i32.gt_u",   0x4B, "numeric_i32", nil,             2, 1},
	{"i32.le_s",   0x4C, "numeric_i32", nil,             2, 1},
	{"i32.le_u",   0x4D, "numeric_i32", nil,             2, 1},
	{"i32.ge_s",   0x4E, "numeric_i32", nil,             2, 1},
	{"i32.ge_u",   0x4F, "numeric_i32", nil,             2, 1},
	{"i32.clz",    0x67, "numeric_i32", nil,             1, 1},
	{"i32.ctz",    0x68, "numeric_i32", nil,             1, 1},
	{"i32.popcnt", 0x69, "numeric_i32", nil,             1, 1},
	{"i32.add",    0x6A, "numeric_i32", nil,             2, 1},
	{"i32.sub",    0x6B, "numeric_i32", nil,             2, 1},
	{"i32.mul",    0x6C, "numeric_i32", nil,             2, 1},
	{"i32.div_s",  0x6D, "numeric_i32", nil,             2, 1},
	{"i32.div_u",  0x6E, "numeric_i32", nil,             2, 1},
	{"i32.rem_s",  0x6F, "numeric_i32", nil,             2, 1},
	{"i32.rem_u",  0x70, "numeric_i32", nil,             2, 1},
	{"i32.and",    0x71, "numeric_i32", nil,             2, 1},
	{"i32.or",     0x72, "numeric_i32", nil,             2, 1},
	{"i32.xor",    0x73, "numeric_i32", nil,             2, 1},
	{"i32.shl",    0x74, "numeric_i32", nil,             2, 1},
	{"i32.shr_s",  0x75, "numeric_i32", nil,             2, 1},
	{"i32.shr_u",  0x76, "numeric_i32", nil,             2, 1},
	{"i32.rotl",   0x77, "numeric_i32", nil,             2, 1},
	{"i32.rotr",   0x78, "numeric_i32", nil,             2, 1},

	// -----------------------------------------------------------------------
	// i64 NUMERIC — 64-bit integer operations
	//
	// The structure mirrors i32. Used for 64-bit counters, timestamps,
	// file offsets, and any integer too large for 32 bits.
	// -----------------------------------------------------------------------
	{"i64.const",  0x42, "numeric_i64", []string{"i64"}, 0, 1},
	{"i64.eqz",    0x50, "numeric_i64", nil,             1, 1},
	{"i64.eq",     0x51, "numeric_i64", nil,             2, 1},
	{"i64.ne",     0x52, "numeric_i64", nil,             2, 1},
	{"i64.lt_s",   0x53, "numeric_i64", nil,             2, 1},
	{"i64.lt_u",   0x54, "numeric_i64", nil,             2, 1},
	{"i64.gt_s",   0x55, "numeric_i64", nil,             2, 1},
	{"i64.gt_u",   0x56, "numeric_i64", nil,             2, 1},
	{"i64.le_s",   0x57, "numeric_i64", nil,             2, 1},
	{"i64.le_u",   0x58, "numeric_i64", nil,             2, 1},
	{"i64.ge_s",   0x59, "numeric_i64", nil,             2, 1},
	{"i64.ge_u",   0x5A, "numeric_i64", nil,             2, 1},
	{"i64.clz",    0x79, "numeric_i64", nil,             1, 1},
	{"i64.ctz",    0x7A, "numeric_i64", nil,             1, 1},
	{"i64.popcnt", 0x7B, "numeric_i64", nil,             1, 1},
	{"i64.add",    0x7C, "numeric_i64", nil,             2, 1},
	{"i64.sub",    0x7D, "numeric_i64", nil,             2, 1},
	{"i64.mul",    0x7E, "numeric_i64", nil,             2, 1},
	{"i64.div_s",  0x7F, "numeric_i64", nil,             2, 1},
	{"i64.div_u",  0x80, "numeric_i64", nil,             2, 1},
	{"i64.rem_s",  0x81, "numeric_i64", nil,             2, 1},
	{"i64.rem_u",  0x82, "numeric_i64", nil,             2, 1},
	{"i64.and",    0x83, "numeric_i64", nil,             2, 1},
	{"i64.or",     0x84, "numeric_i64", nil,             2, 1},
	{"i64.xor",    0x85, "numeric_i64", nil,             2, 1},
	{"i64.shl",    0x86, "numeric_i64", nil,             2, 1},
	{"i64.shr_s",  0x87, "numeric_i64", nil,             2, 1},
	{"i64.shr_u",  0x88, "numeric_i64", nil,             2, 1},
	{"i64.rotl",   0x89, "numeric_i64", nil,             2, 1},
	{"i64.rotr",   0x8A, "numeric_i64", nil,             2, 1},

	// -----------------------------------------------------------------------
	// f32 NUMERIC — 32-bit IEEE 754 floating-point operations
	//
	// "f32.const" encodes a 32-bit float as 4 raw bytes (little-endian IEEE 754).
	// Unlike integer constants (which use variable-length LEB128), floats use
	// a fixed 4-byte encoding to preserve the exact bit pattern.
	//
	// "nearest" rounds to the nearest even integer (IEEE 754 default mode,
	// also called "banker's rounding"). This differs from C's round() which
	// rounds half away from zero.
	//
	// "copysign(a, b)" = magnitude of a with sign bit of b.
	// -----------------------------------------------------------------------
	{"f32.const",    0x43, "numeric_f32", []string{"f32"}, 0, 1},
	{"f32.eq",       0x5B, "numeric_f32", nil,             2, 1},
	{"f32.ne",       0x5C, "numeric_f32", nil,             2, 1},
	{"f32.lt",       0x5D, "numeric_f32", nil,             2, 1},
	{"f32.gt",       0x5E, "numeric_f32", nil,             2, 1},
	{"f32.le",       0x5F, "numeric_f32", nil,             2, 1},
	{"f32.ge",       0x60, "numeric_f32", nil,             2, 1},
	{"f32.abs",      0x8B, "numeric_f32", nil,             1, 1},
	{"f32.neg",      0x8C, "numeric_f32", nil,             1, 1},
	{"f32.ceil",     0x8D, "numeric_f32", nil,             1, 1},
	{"f32.floor",    0x8E, "numeric_f32", nil,             1, 1},
	{"f32.trunc",    0x8F, "numeric_f32", nil,             1, 1},
	{"f32.nearest",  0x90, "numeric_f32", nil,             1, 1},
	{"f32.sqrt",     0x91, "numeric_f32", nil,             1, 1},
	{"f32.add",      0x92, "numeric_f32", nil,             2, 1},
	{"f32.sub",      0x93, "numeric_f32", nil,             2, 1},
	{"f32.mul",      0x94, "numeric_f32", nil,             2, 1},
	{"f32.div",      0x95, "numeric_f32", nil,             2, 1},
	{"f32.min",      0x96, "numeric_f32", nil,             2, 1},
	{"f32.max",      0x97, "numeric_f32", nil,             2, 1},
	{"f32.copysign", 0x98, "numeric_f32", nil,             2, 1},

	// -----------------------------------------------------------------------
	// f64 NUMERIC — 64-bit IEEE 754 floating-point operations
	//
	// "f64.const" encodes a 64-bit double as 8 raw bytes (little-endian IEEE 754).
	// The operation set mirrors f32 exactly.
	// -----------------------------------------------------------------------
	{"f64.const",    0x44, "numeric_f64", []string{"f64"}, 0, 1},
	{"f64.eq",       0x61, "numeric_f64", nil,             2, 1},
	{"f64.ne",       0x62, "numeric_f64", nil,             2, 1},
	{"f64.lt",       0x63, "numeric_f64", nil,             2, 1},
	{"f64.gt",       0x64, "numeric_f64", nil,             2, 1},
	{"f64.le",       0x65, "numeric_f64", nil,             2, 1},
	{"f64.ge",       0x66, "numeric_f64", nil,             2, 1},
	{"f64.abs",      0x99, "numeric_f64", nil,             1, 1},
	{"f64.neg",      0x9A, "numeric_f64", nil,             1, 1},
	{"f64.ceil",     0x9B, "numeric_f64", nil,             1, 1},
	{"f64.floor",    0x9C, "numeric_f64", nil,             1, 1},
	{"f64.trunc",    0x9D, "numeric_f64", nil,             1, 1},
	{"f64.nearest",  0x9E, "numeric_f64", nil,             1, 1},
	{"f64.sqrt",     0x9F, "numeric_f64", nil,             1, 1},
	{"f64.add",      0xA0, "numeric_f64", nil,             2, 1},
	{"f64.sub",      0xA1, "numeric_f64", nil,             2, 1},
	{"f64.mul",      0xA2, "numeric_f64", nil,             2, 1},
	{"f64.div",      0xA3, "numeric_f64", nil,             2, 1},
	{"f64.min",      0xA4, "numeric_f64", nil,             2, 1},
	{"f64.max",      0xA5, "numeric_f64", nil,             2, 1},
	{"f64.copysign", 0xA6, "numeric_f64", nil,             2, 1},

	// -----------------------------------------------------------------------
	// CONVERSIONS — type conversions between numeric types
	//
	// All conversions pop 1 value and push 1 value (different type).
	// No immediates — all information is in the opcode itself.
	//
	// Naming convention: result_type.operation_source_type
	//
	//   "wrap"        — i32.wrap_i64: keep only the low 32 bits of an i64.
	//                   The high 32 bits are silently discarded.
	//                   Example: 0x0000_0001_FFFF_FFFF → 0xFFFF_FFFF (= -1)
	//
	//   "extend"      — i64.extend_i32_s/u: widen an i32 to 64 bits.
	//                   _s: sign-extend (copy the sign bit into high 32 bits)
	//                   _u: zero-extend (fill high 32 bits with 0)
	//
	//   "trunc"       — i32.trunc_f32_s: truncate a float to integer (toward zero).
	//                   Traps if the float is NaN or overflows the int range.
	//
	//   "convert"     — f32.convert_i32_s: convert integer to float.
	//                   May lose precision for large integers.
	//
	//   "demote"      — f32.demote_f64: narrow f64 to f32 (may lose precision).
	//   "promote"     — f64.promote_f32: widen f32 to f64 (lossless).
	//
	//   "reinterpret" — i32.reinterpret_f32: reinterpret the same N bits as a
	//                   different type. No numeric conversion. Like a C union.
	//                   Example: f32 1.0 → i32 0x3F800000 (the IEEE 754 bits)
	// -----------------------------------------------------------------------
	{"i32.wrap_i64",        0xA7, "conversion", nil, 1, 1},
	{"i32.trunc_f32_s",     0xA8, "conversion", nil, 1, 1},
	{"i32.trunc_f32_u",     0xA9, "conversion", nil, 1, 1},
	{"i32.trunc_f64_s",     0xAA, "conversion", nil, 1, 1},
	{"i32.trunc_f64_u",     0xAB, "conversion", nil, 1, 1},
	{"i64.extend_i32_s",    0xAC, "conversion", nil, 1, 1},
	{"i64.extend_i32_u",    0xAD, "conversion", nil, 1, 1},
	{"i64.trunc_f32_s",     0xAE, "conversion", nil, 1, 1},
	{"i64.trunc_f32_u",     0xAF, "conversion", nil, 1, 1},
	{"i64.trunc_f64_s",     0xB0, "conversion", nil, 1, 1},
	{"i64.trunc_f64_u",     0xB1, "conversion", nil, 1, 1},
	{"f32.convert_i32_s",   0xB2, "conversion", nil, 1, 1},
	{"f32.convert_i32_u",   0xB3, "conversion", nil, 1, 1},
	{"f32.convert_i64_s",   0xB4, "conversion", nil, 1, 1},
	{"f32.convert_i64_u",   0xB5, "conversion", nil, 1, 1},
	{"f32.demote_f64",      0xB6, "conversion", nil, 1, 1},
	{"f64.convert_i32_s",   0xB7, "conversion", nil, 1, 1},
	{"f64.convert_i32_u",   0xB8, "conversion", nil, 1, 1},
	{"f64.convert_i64_s",   0xB9, "conversion", nil, 1, 1},
	{"f64.convert_i64_u",   0xBA, "conversion", nil, 1, 1},
	{"f64.promote_f32",     0xBB, "conversion", nil, 1, 1},
	{"i32.reinterpret_f32", 0xBC, "conversion", nil, 1, 1},
	{"i64.reinterpret_f64", 0xBD, "conversion", nil, 1, 1},
	{"f32.reinterpret_i32", 0xBE, "conversion", nil, 1, 1},
	{"f64.reinterpret_i64", 0xBF, "conversion", nil, 1, 1},
}

// ---------------------------------------------------------------------------
// LOOKUP MAPS
//
// Opcodes maps each byte value to its OpcodeInfo.
// OpcodesByName maps each mnemonic string to its OpcodeInfo.
//
// Both maps are populated exactly once at package initialization by init().
// After init() returns, both maps are effectively read-only — callers must
// not modify them.
//
// Map type choice: map[byte]OpcodeInfo rather than map[int]OpcodeInfo because
// WASM opcodes are always a single byte (0x00–0xFF). Using byte makes the
// type signature self-documenting and prevents accidentally passing a
// multi-byte value.
// ---------------------------------------------------------------------------

// Opcodes maps opcode byte values to their OpcodeInfo.
// Keyed by the single-byte opcode value.
//
// Example:
//
//	info, ok := Opcodes[0x6A]
//	// info.Name == "i32.add"
//
//nolint:gochecknoglobals
var Opcodes map[byte]OpcodeInfo

// OpcodesByName maps mnemonic strings to their OpcodeInfo.
// Keyed by the WASM text-format name.
//
// Example:
//
//	info, ok := OpcodesByName["i32.add"]
//	// info.Opcode == 0x6A
//
//nolint:gochecknoglobals
var OpcodesByName map[string]OpcodeInfo

// init builds both lookup maps from the raw table at package startup.
// This runs exactly once, before any other code in this package executes.
func init() {
	Opcodes = make(map[byte]OpcodeInfo, len(rawTable))
	OpcodesByName = make(map[string]OpcodeInfo, len(rawTable))
	for _, e := range rawTable {
		info := OpcodeInfo{
			Name:       e.name,
			Opcode:     e.opcode,
			Category:   e.category,
			Immediates: e.immediates,
			StackPop:   e.stackPop,
			StackPush:  e.stackPush,
		}
		Opcodes[e.opcode] = info
		OpcodesByName[e.name] = info
	}
}

// ---------------------------------------------------------------------------
// CONVENIENCE FUNCTIONS
//
// These functions return a pointer to OpcodeInfo and a boolean found flag,
// following Go idiom (rather than panic or returning a zero value on miss).
// Returning a pointer lets callers distinguish "not found" (nil) from a
// found entry without needing to check the bool separately.
//
// Alternative idiom used below: return (OpcodeInfo, bool) — this is idiomatic
// Go for map-lookup wrappers (matches the two-value map indexing syntax).
//
// Both functions are O(1) — they delegate directly to Go's built-in map,
// which is a hash table.
// ---------------------------------------------------------------------------

// GetOpcode returns the OpcodeInfo for the given byte value and true,
// or an empty OpcodeInfo and false if the byte is not a known WASM 1.0 opcode.
//
// Example:
//
//	info, ok := GetOpcode(0x6A)
//	// ok == true, info.Name == "i32.add"
//
//	_, ok = GetOpcode(0xFF)
//	// ok == false (0xFF is not a valid WASM 1.0 opcode)
func GetOpcode(opcode byte) (OpcodeInfo, bool) {
	info, ok := Opcodes[opcode]
	return info, ok
}

// GetOpcodeByName returns the OpcodeInfo for the given mnemonic and true,
// or an empty OpcodeInfo and false if the name is not a known WASM 1.0
// instruction.
//
// Example:
//
//	info, ok := GetOpcodeByName("i32.add")
//	// ok == true, info.Opcode == 0x6A
//
//	_, ok = GetOpcodeByName("not_real")
//	// ok == false
func GetOpcodeByName(name string) (OpcodeInfo, bool) {
	info, ok := OpcodesByName[name]
	return info, ok
}
