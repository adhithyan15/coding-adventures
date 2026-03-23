// Package wasmtypes provides the WASM 1.0 type system: pure data structures
// representing every type-level concept in the WebAssembly binary format.
//
// This package is part of the coding-adventures monorepo, a ground-up
// implementation of the computing stack from transistors to operating systems.
//
// # Background: Where Types Live in the WASM Binary
//
// A WebAssembly module is a sequence of named sections. Each section is tagged
// with a 1-byte ID and holds a specific kind of structured data:
//
//	┌────────────┬──────────────────────┬─────────────────────────────────┐
//	│ Section ID │ Section Name         │ Go type (this package)          │
//	├────────────┼──────────────────────┼─────────────────────────────────┤
//	│     1      │ Type section         │ []FuncType                      │
//	│     2      │ Import section       │ []Import                        │
//	│     3      │ Function section     │ []uint32  (type indices)        │
//	│     4      │ Table section        │ []TableType                     │
//	│     5      │ Memory section       │ []MemoryType                    │
//	│     6      │ Global section       │ []Global                        │
//	│     7      │ Export section       │ []Export                        │
//	│     8      │ Start section        │ *uint32   (nil if absent)       │
//	│     9      │ Element section      │ []Element                       │
//	│    10      │ Code section         │ []FunctionBody                  │
//	│    11      │ Data section         │ []DataSegment                   │
//	│     0      │ Custom sections      │ []CustomSection                 │
//	└────────────┴──────────────────────┴─────────────────────────────────┘
//
// The structures in this package correspond 1-to-1 to those rows. Byte values
// of enum constants match the WASM binary encoding, so the parser can use
// them directly without translation.
//
// # Value Type Encoding
//
// WASM's four primitive value types are encoded as single-byte values in the
// range 0x7C–0x7F. Interpreted as signed LEB128, these are -4 through -1.
// This trick lets the binary format distinguish value types (negative) from
// type indices (non-negative) in contexts where either is valid.
//
//	┌─────────────────────────────────────────┐
//	│  Byte  │ Type  │ Signed LEB128 value     │
//	├────────┼───────┼─────────────────────────┤
//	│  0x7F  │  i32  │  -1                     │
//	│  0x7E  │  i64  │  -2                     │
//	│  0x7D  │  f32  │  -3                     │
//	│  0x7C  │  f64  │  -4                     │
//	└─────────────────────────────────────────┘
//
// # Design Choices
//
//  1. ValueType and ExternalKind are declared as named byte types with
//     package-level constants. iota is NOT used because the values are
//     specific hex literals from the spec and must not change.
//
//  2. Structs are plain Go structs (no frozen concept). Go callers are
//     expected not to mutate structs after construction; the type system
//     does not enforce immutability at the language level.
//
//  3. WasmModule uses slice fields. The zero value (nil slice) is valid —
//     a nil slice behaves identically to an empty slice for iteration and
//     appending.
//
//  4. The optional start function index uses *uint32 (pointer). nil means
//     the Start section is absent; a non-nil pointer holds the index.
package wasmtypes

// ---------------------------------------------------------------------------
// VALUE TYPE
//
// ValueType is the type of a value on the WASM operand stack, in function
// signatures, local declarations, and global variable types.
//
// The four constants below use their WASM binary byte values directly.
// They are chosen to be in the range 0x7C–0x7F because:
//  - In unsigned interpretation: 124–127 (high byte values, out of ASCII range)
//  - In signed LEB128:           -4 through -1 (negative, distinct from type
//    indices which are always >= 0)
//
// Binary layout of a ValueType byte:
//
//	Bit: 7   6   5   4   3   2   1   0
//	     0   1   1   1   1   1   x   x
//	                             ↑   ↑
//	                           differentiates i32/i64/f32/f64
//
// Example — a FuncType with params (i32, f64) and result (i64):
//
//	0x60           functype indicator
//	0x02           2 params
//	0x7F           i32
//	0x7C           f64
//	0x01           1 result
//	0x7E           i64
// ---------------------------------------------------------------------------

// ValueType names the type of a WASM value on the operand stack.
type ValueType byte

const (
	// ValueTypeI32 is a 32-bit integer (also used for booleans and addresses).
	ValueTypeI32 ValueType = 0x7F

	// ValueTypeI64 is a 64-bit integer.
	ValueTypeI64 ValueType = 0x7E

	// ValueTypeF32 is a 32-bit IEEE 754 single-precision float.
	ValueTypeF32 ValueType = 0x7D

	// ValueTypeF64 is a 64-bit IEEE 754 double-precision float.
	ValueTypeF64 ValueType = 0x7C
)

// ---------------------------------------------------------------------------
// BLOCK TYPE
//
// A block type encodes the result signature of a structured control flow
// block (block, loop, if). Three cases exist:
//
//   case 1 — 0x40: the block produces no values (EMPTY)
//   case 2 — 0x7C..0x7F: the block produces exactly one value (ValueType)
//   case 3 — signed LEB128 integer >= 0: type index for multi-value results
//
// This package defines only the EMPTY constant. For case 2, use a ValueType
// constant directly. For case 3, use a plain int32.
//
// Encoding table:
//
//	┌────────────┬──────────────────────────────────────────────────────┐
//	│ Byte range │ Meaning                                               │
//	├────────────┼──────────────────────────────────────────────────────┤
//	│   0x40     │ no result values (BlockTypeEmpty)                     │
//	│  0x7C–0x7F │ single result (use ValueTypeI32/I64/F32/F64)         │
//	│  >= 0x00   │ type section index for multi-value results (int32)    │
//	└────────────┴──────────────────────────────────────────────────────┘
// ---------------------------------------------------------------------------

// BlockType encodes the result arity of a WASM structured control block.
type BlockType byte

const (
	// BlockTypeEmpty means the block produces no result values.
	// Encoded as 0x40 in the binary format.
	BlockTypeEmpty BlockType = 0x40
)

// ---------------------------------------------------------------------------
// EXTERNAL KIND
//
// ExternalKind identifies whether an Import or Export refers to a function,
// table, memory, or global variable.
//
// Binary encoding: a single byte 0x00–0x03, immediately following the name
// strings in an Import entry or Export entry.
//
//	┌──────┬──────────┬────────────────────────────────────────────────┐
//	│ Byte │ Kind     │ Description                                     │
//	├──────┼──────────┼────────────────────────────────────────────────┤
//	│ 0x00 │ FUNCTION │ A callable function (by type index)             │
//	│ 0x01 │ TABLE    │ A table of opaque references (funcref in 1.0)   │
//	│ 0x02 │ MEMORY   │ Linear memory (array of bytes, page-granular)   │
//	│ 0x03 │ GLOBAL   │ A global variable (value + mutability)          │
//	└──────┴──────────┴────────────────────────────────────────────────┘
//
// Example (WASI fd_write import):
//
//	module = "wasi_snapshot_preview1"
//	name   = "fd_write"
//	kind   = ExternalKindFunction
//	index  = 3  (type section index of the function signature)
// ---------------------------------------------------------------------------

// ExternalKind classifies what a WASM import or export refers to.
type ExternalKind byte

const (
	// ExternalKindFunction refers to a callable function.
	ExternalKindFunction ExternalKind = 0x00

	// ExternalKindTable refers to a table of opaque references.
	ExternalKindTable ExternalKind = 0x01

	// ExternalKindMemory refers to linear memory (64 KiB pages).
	ExternalKindMemory ExternalKind = 0x02

	// ExternalKindGlobal refers to a global variable.
	ExternalKindGlobal ExternalKind = 0x03
)

// ---------------------------------------------------------------------------
// FUNC TYPE
//
// FuncType describes a function's calling convention: the types of its
// parameter values and result values. Multiple FuncType entries are stored
// in the Type section (section ID = 1), indexed from zero.
//
// Binary encoding of a single FuncType entry:
//
//	0x60                functype indicator byte
//	<LEB128 count>      number of parameter types
//	<ValueType bytes>   one byte per parameter (left to right)
//	<LEB128 count>      number of result types
//	<ValueType bytes>   one byte per result
//
// Example — (i32, i64) -> (f64):
//
//	0x60  0x02  0x7F 0x7E  0x01  0x7C
//	^^^   ^^^   ^^^^^^^^^   ^^^   ^^^
//	func  2par  i32  i64    1res  f64
//
// The Function section (section ID = 3) stores only type indices (uint32),
// one per locally defined function, pointing back into this type table.
// This avoids repeating the full type for each function.
// ---------------------------------------------------------------------------

// FuncType is a WASM function type: its parameter types and result types.
//
// In WASM 1.0, a function has at most one result value. WASM 1.1 added
// the multi-value extension that allows multiple results, but the type
// encoding was designed from the start to accommodate it.
//
// Example:
//
//	ft := FuncType{Params: []ValueType{ValueTypeI32}, Results: []ValueType{ValueTypeI64}}
//	// Represents: (i32) -> (i64)
type FuncType struct {
	Params  []ValueType // parameter types (left to right in the function signature)
	Results []ValueType // result types (usually 0 or 1 in WASM 1.0)
}

// ---------------------------------------------------------------------------
// LIMITS
//
// Limits constrains the size of a memory or table. The minimum is always
// required; the maximum is optional (represented by a boolean flag HasMax).
//
// Binary encoding:
//
//	0x00  <min>           only minimum (no upper bound)
//	0x01  <min>  <max>    minimum and maximum
//
// Where <min> and <max> are unsigned LEB128 integers.
//
// For memory: the unit is 64-KiB pages (1 page = 65536 bytes).
//             WASM 1.0 allows up to 65536 pages = 4 GiB total.
// For tables: the unit is element slots (opaque references).
//
// Example — 1 initial page, up to 4 pages:
//
//	0x01  0x01  0x04
//	^^^   ^^^   ^^^
//	has-max  1   4
//
// Example — unbounded growth starting from 0 pages:
//
//	0x00  0x00
//	^^^   ^^^
//	no-max  0
// ---------------------------------------------------------------------------

// Limits constrains the minimum and optional maximum size of a memory or table.
//
// Example:
//
//	lim := Limits{Min: 1, Max: 4, HasMax: true}   // at least 1, at most 4
//	lim := Limits{Min: 0}                          // unbounded (no max)
type Limits struct {
	Min    uint32 // minimum size (required)
	Max    uint32 // maximum size (valid only when HasMax is true)
	HasMax bool   // true if an explicit maximum was specified
}

// ---------------------------------------------------------------------------
// MEMORY TYPE
//
// MemoryType describes a linear memory. WASM 1.0 allows at most one memory
// per module. The memory is a flat byte array addressed by i32 values.
// Its size is measured in 64-KiB pages.
//
// Binary encoding (Memory section):
//
//	<limits>    — Limits encoding (0x00/<min> or 0x01/<min>/<max>)
//
// Example — 1 page minimum, no maximum:
//
//	0x00  0x01
//	^^^   ^^^
//	no-max  1
// ---------------------------------------------------------------------------

// MemoryType describes a WASM linear memory's size constraints.
//
// Example:
//
//	mt := MemoryType{Limits: Limits{Min: 1, Max: 4, HasMax: true}}
type MemoryType struct {
	Limits Limits
}

// ---------------------------------------------------------------------------
// TABLE TYPE
//
// A table is an array of opaque references. In WASM 1.0, the only valid
// element type is 0x70 = funcref — a reference to a function. Tables are
// used by the call_indirect instruction for indirect function dispatch.
//
// Binary encoding (Table section):
//
//	<element_type>   0x70 = funcref
//	<limits>         Limits encoding
//
// Example — table with 1 to 16 elements:
//
//	0x70  0x01  0x01  0x10
//	^^^   ^^^   ^^^   ^^^^
//	fref  has-max  1   16
// ---------------------------------------------------------------------------

// ElementTypeFuncRef is the only valid table element type in WASM 1.0.
// Value 0x70 = funcref (an opaque reference to a function).
const ElementTypeFuncRef byte = 0x70

// TableType describes a WASM table's element type and size constraints.
//
// Example:
//
//	tt := TableType{ElementType: ElementTypeFuncRef, Limits: Limits{Min: 0, Max: 100, HasMax: true}}
type TableType struct {
	ElementType byte   // reference type; 0x70 = funcref (only valid value in WASM 1.0)
	Limits      Limits // minimum and optional maximum number of elements
}

// ---------------------------------------------------------------------------
// GLOBAL TYPE
//
// A GlobalType describes a global variable: its value type and mutability.
//
// Binary encoding:
//
//	<value_type>   one byte (ValueType)
//	<mutability>   0x00 = immutable, 0x01 = mutable
//
// Example — mutable i32 (typical use: linear stack pointer):
//
//	0x7F  0x01
//	^^^   ^^^
//	i32   mutable
//
// Example — immutable i32 (typical use: constant like __data_end):
//
//	0x7F  0x00
//	^^^   ^^^
//	i32   immutable
// ---------------------------------------------------------------------------

// GlobalType describes the type and mutability of a WASM global variable.
//
// Example:
//
//	gt := GlobalType{ValueType: ValueTypeI32, Mutable: true}  // mutable i32
//	gt := GlobalType{ValueType: ValueTypeF64, Mutable: false} // immutable f64
type GlobalType struct {
	ValueType ValueType // the type of the global's stored value
	Mutable   bool      // true if set_global is allowed; false if read-only
}

// ---------------------------------------------------------------------------
// IMPORT
//
// An Import declares an external entity that the host must supply. WASM
// modules can import functions, tables, memories, and globals.
//
// Binary encoding (one entry in the Import section):
//
//	<module_len>   LEB128 byte count of module name
//	<module_name>  UTF-8 bytes
//	<name_len>     LEB128 byte count of entity name
//	<name>         UTF-8 bytes
//	<kind>         1 byte (ExternalKind)
//	<type_info>    depends on kind:
//	                 FUNCTION → LEB128 type index (uint32)
//	                 TABLE    → TableType encoding
//	                 MEMORY   → MemoryType encoding (= Limits encoding)
//	                 GLOBAL   → GlobalType encoding
//
// Because Go doesn't have sum types, ImportTypeInfo is an interface{}
// (any). The caller casts using type assertions:
//
//	if idx, ok := imp.TypeInfo.(uint32); ok { /* function import */ }
//	if tt, ok := imp.TypeInfo.(TableType); ok { /* table import */ }
//	if mt, ok := imp.TypeInfo.(MemoryType); ok { /* memory import */ }
//	if gt, ok := imp.TypeInfo.(GlobalType); ok { /* global import */ }
// ---------------------------------------------------------------------------

// Import is a single import entry in the WASM Import section.
//
// TypeInfo holds one of: uint32 (function type index), TableType, MemoryType,
// or GlobalType, depending on Kind.
//
// Example (WASI memory import):
//
//	imp := Import{
//	    ModuleName: "wasi_snapshot_preview1",
//	    Name:       "memory",
//	    Kind:       ExternalKindMemory,
//	    TypeInfo:   MemoryType{Limits: Limits{Min: 1}},
//	}
type Import struct {
	ModuleName string       // the module namespace (e.g., "env")
	Name       string       // the entity name within the module
	Kind       ExternalKind // what kind of entity is imported
	TypeInfo   any          // uint32 | TableType | MemoryType | GlobalType
}

// ---------------------------------------------------------------------------
// EXPORT
//
// An Export makes a module's internal entity accessible from outside.
// Exports are identified by a name and point to an index in the appropriate
// address space for their kind.
//
// Binary encoding (one entry in the Export section):
//
//	<name_len>   LEB128
//	<name>       UTF-8
//	<kind>       1 byte (ExternalKind)
//	<index>      LEB128 (uint32)
//
// Index interpretation by kind:
//   - FUNCTION: index in the function address space (imported + local)
//   - TABLE:    index in the table space
//   - MEMORY:   index in the memory space (always 0 in WASM 1.0)
//   - GLOBAL:   index in the global space (imported + local)
//
// Example — exporting a "main" function at index 5:
//
//	exp := Export{Name: "main", Kind: ExternalKindFunction, Index: 5}
// ---------------------------------------------------------------------------

// Export is a single export entry in the WASM Export section.
//
// Example:
//
//	exp := Export{Name: "_start", Kind: ExternalKindFunction, Index: 0}
type Export struct {
	Name  string       // the export name visible to the host
	Kind  ExternalKind // what kind of entity is exported
	Index uint32       // index into the corresponding address space
}

// ---------------------------------------------------------------------------
// GLOBAL
//
// A module-defined global variable. Combines a GlobalType (type + mutability)
// with a constant initializer expression. The initializer is a short sequence
// of bytecode that the WASM runtime evaluates once at module instantiation.
//
// Binary encoding (one entry in the Global section):
//
//	<global_type>   GlobalType encoding (value_type + mutability)
//	<init_expr>     constant expression bytes, terminated by 0x0B (end)
//
// Valid initializer opcodes (constant expressions only):
//
//	i32.const N:  0x41 <leb128(N)> 0x0B
//	i64.const N:  0x42 <leb128(N)> 0x0B
//	f32.const N:  0x43 <4 bytes>   0x0B
//	f64.const N:  0x44 <8 bytes>   0x0B
//	global.get K: 0x23 <leb128(K)> 0x0B  (only for immutable imports)
//
// Example — mutable i32 global initialized to 42:
//
//	g := Global{
//	    GlobalType: GlobalType{ValueType: ValueTypeI32, Mutable: true},
//	    InitExpr:   []byte{0x41, 0x2A, 0x0B},   // i32.const 42; end
//	}
// ---------------------------------------------------------------------------

// Global is a module-defined WASM global variable with its initializer.
//
// Example:
//
//	g := Global{
//	    GlobalType: GlobalType{ValueType: ValueTypeI32, Mutable: false},
//	    InitExpr:   []byte{0x41, 0x00, 0x0B},   // i32.const 0; end
//	}
type Global struct {
	GlobalType GlobalType // type and mutability of this global
	InitExpr   []byte     // constant initializer expression (ends with 0x0B)
}

// ---------------------------------------------------------------------------
// ELEMENT
//
// An element segment initializes table entries with function references.
// It specifies the target table, a byte offset into that table (computed
// by a constant expression), and a list of function indices.
//
// Binary encoding (one entry in the Element section):
//
//	<table_index>    LEB128 (uint32) — always 0 in WASM 1.0
//	<offset_expr>    constant expression (i32.const + end = 3 bytes typically)
//	<count>          LEB128 number of function indices
//	<func_indices>   one LEB128 uint32 per index
//
// Example — fill table[2..4] with functions 10 and 11:
//
//	elem := Element{
//	    TableIndex:       0,
//	    OffsetExpr:       []byte{0x41, 0x02, 0x0B},  // i32.const 2; end
//	    FunctionIndices:  []uint32{10, 11},
//	}
// ---------------------------------------------------------------------------

// Element is a WASM element segment: initializes table slots with function refs.
//
// Example:
//
//	elem := Element{
//	    TableIndex:      0,
//	    OffsetExpr:      []byte{0x41, 0x00, 0x0B},
//	    FunctionIndices: []uint32{1, 2, 3},
//	}
type Element struct {
	TableIndex      uint32   // which table to initialize (always 0 in WASM 1.0)
	OffsetExpr      []byte   // constant expression: byte offset in the table
	FunctionIndices []uint32 // function indices to write into the table slots
}

// ---------------------------------------------------------------------------
// DATA SEGMENT
//
// A data segment initializes a region of linear memory with a known byte
// string. The module instantiation process writes these bytes into memory
// at the specified offset before any code executes.
//
// Binary encoding (one entry in the Data section):
//
//	<memory_index>  LEB128 (uint32) — always 0 in WASM 1.0
//	<offset_expr>   constant expression bytes
//	<count>         LEB128 byte length of data
//	<data>          raw bytes
//
// Example — write the string "hello" at memory byte offset 256:
//
//	ds := DataSegment{
//	    MemoryIndex: 0,
//	    OffsetExpr:  []byte{0x41, 0x80, 0x02, 0x0B},  // i32.const 256; end
//	    Data:        []byte("hello"),
//	}
// ---------------------------------------------------------------------------

// DataSegment is a WASM data segment: initializes a range of linear memory.
//
// Example:
//
//	ds := DataSegment{
//	    MemoryIndex: 0,
//	    OffsetExpr:  []byte{0x41, 0x00, 0x0B},
//	    Data:        []byte("hello, wasm"),
//	}
type DataSegment struct {
	MemoryIndex uint32 // which memory to initialize (always 0 in WASM 1.0)
	OffsetExpr  []byte // constant expression: byte offset in memory
	Data        []byte // raw bytes to write at that offset
}

// ---------------------------------------------------------------------------
// FUNCTION BODY
//
// A function body is the executable code of a locally-defined function.
// It declares the types of additional local variables (beyond the function's
// parameters) and contains the raw bytecode instructions.
//
// Parameters are NOT listed in FunctionBody — they are declared in the
// corresponding FuncType and are accessible as locals 0..len(params)-1.
// The additional locals listed here start at index len(params).
//
// Binary encoding (one entry in the Code section):
//
//	<body_size>    LEB128 total byte count of this entry (excluding this field)
//	<local_count>  LEB128 number of local variable groups
//	               (each group: count:LEB128, type:ValueType byte)
//	<code>         raw bytecode bytes, ending with opcode 0x0B (end)
//
// In this struct, Locals stores one ValueType per local (already expanded
// from the group encoding). Code holds the raw bytecode.
//
// Example — function body with 2 i32 locals that adds them:
//
//	fb := FunctionBody{
//	    Locals: []ValueType{ValueTypeI32, ValueTypeI32},
//	    Code:   []byte{0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B},
//	//               local.get 0  local.get 1  i32.add  end
//	}
// ---------------------------------------------------------------------------

// FunctionBody holds the executable body of a locally-defined WASM function.
//
// Example:
//
//	fb := FunctionBody{
//	    Locals: []ValueType{ValueTypeI32},
//	    Code:   []byte{0x20, 0x00, 0x0B},   // local.get 0; end
//	}
type FunctionBody struct {
	Locals []ValueType // additional local variable types (beyond parameters)
	Code   []byte      // raw bytecode, ending with 0x0B (end opcode)
}

// ---------------------------------------------------------------------------
// CUSTOM SECTION
//
// Custom sections carry implementation-defined data outside the WASM spec.
// The WASM runtime ignores any custom section it does not recognize, making
// them a safe extension point for tools, debuggers, and linkers.
//
// Common custom section names:
//   - "name":       WASM Name section — human-readable names for functions
//   - ".debug_info": DWARF debug info (used by Emscripten, wasm-pack)
//   - "producers":  Toolchain metadata (compiler name, version)
//
// Binary encoding:
//
//	0x00              section ID = 0 (custom)
//	<section_size>    LEB128 total byte count
//	<name_len>        LEB128 byte count of section name
//	<name>            UTF-8 section name
//	<data>            arbitrary payload bytes
// ---------------------------------------------------------------------------

// CustomSection is a WASM custom section: a named blob of arbitrary data.
//
// Example:
//
//	cs := CustomSection{Name: "name", Data: []byte{0x00, 0x04, 'm', 'a', 'i', 'n'}}
type CustomSection struct {
	Name string // section name (e.g., "name", "producers")
	Data []byte // arbitrary payload bytes
}

// ---------------------------------------------------------------------------
// WASM MODULE
//
// WasmModule is the top-level container for a parsed WebAssembly binary.
// It holds one slice per section type, mirroring the module structure.
//
// A parser fills this struct incrementally as it reads section by section.
// All slice fields default to nil (zero value), which is equivalent to an
// empty slice for appending and ranging.
//
// Function address space note:
//   The global function index space merges imported and locally-defined
//   functions: indices 0..len(imported_functions)-1 are imports, and
//   indices len(imports)..len(imports)+len(Functions)-1 are local functions.
//   The same pattern applies to tables, memories, and globals.
//
// Example — building a minimal module programmatically:
//
//	m := WasmModule{}
//	m.Types = append(m.Types, FuncType{Params: []ValueType{ValueTypeI32}, Results: nil})
//	m.Functions = append(m.Functions, 0)   // local function uses type 0
//	m.Exports = append(m.Exports, Export{Name: "main", Kind: ExternalKindFunction, Index: 0})
// ---------------------------------------------------------------------------

// WasmModule is the top-level container for a parsed WASM 1.0 binary module.
//
// All slice fields are nil by default (zero value). Parsers append to them
// as sections are read. The Start field uses a pointer so nil can represent
// the absence of the Start section.
//
// Example:
//
//	m := WasmModule{}
//	m.Types = append(m.Types, FuncType{Params: nil, Results: nil})
//	m.Functions = append(m.Functions, 0)
type WasmModule struct {
	Types     []FuncType     // Type section: function signatures (indexed from 0)
	Imports   []Import       // Import section: externally supplied entities
	Functions []uint32       // Function section: type index per local function
	Tables    []TableType    // Table section: table definitions
	Memories  []MemoryType   // Memory section: memory definitions
	Globals   []Global       // Global section: global variable definitions
	Exports   []Export       // Export section: exposed entities
	Start     *uint32        // Start section: entry-point function index (nil if absent)
	Elements  []Element      // Element section: table initialization segments
	Code      []FunctionBody // Code section: function bodies (parallel to Functions)
	Data      []DataSegment  // Data section: memory initialization segments
	Customs   []CustomSection // Custom sections (0 or more, any position)
}
