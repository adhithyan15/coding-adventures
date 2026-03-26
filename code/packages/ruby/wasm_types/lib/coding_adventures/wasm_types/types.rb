# frozen_string_literal: true

# types.rb — Pure type definitions for the WebAssembly 1.0 type system
#
# ─────────────────────────────────────────────────────────────────────────────
# WHAT IS THE WASM TYPE SYSTEM?
# ─────────────────────────────────────────────────────────────────────────────
#
# WebAssembly (WASM) is a binary instruction format for a stack-based virtual
# machine. Every value, function, memory region, and import/export in a WASM
# module is described by a type. This file is the source of truth for all those
# type definitions.
#
# The WASM 1.0 type system is intentionally minimal:
#   - 4 numeric value types (i32, i64, f32, f64)
#   - composite types: FuncType, TableType, MemoryType, GlobalType
#   - structural types: Import, Export, Global, Element, DataSegment,
#                       FunctionBody, CustomSection
#   - a top-level module container: WasmModule
#
# ─────────────────────────────────────────────────────────────────────────────
# THE WASM BINARY SECTION LAYOUT
# ─────────────────────────────────────────────────────────────────────────────
#
# A valid .wasm file looks like this at the top level:
#
#   ┌────────────────────────────────────────────────────────────────────┐
#   │  Magic bytes: 0x00 0x61 0x73 0x6D  ("\0asm")                      │
#   │  Version:     0x01 0x00 0x00 0x00  (1)                            │
#   ├────────────────────────────────────────────────────────────────────┤
#   │  Section 1:  Type section    → FuncType[]                         │
#   │  Section 2:  Import section  → Import[]                           │
#   │  Section 3:  Function section→ type-index[] (one per function)    │
#   │  Section 4:  Table section   → TableType[]                        │
#   │  Section 5:  Memory section  → MemoryType[]                       │
#   │  Section 6:  Global section  → Global[]                           │
#   │  Section 7:  Export section  → Export[]                           │
#   │  Section 8:  Start section   → function index (optional)          │
#   │  Section 9:  Element section → Element[]                          │
#   │  Section 10: Code section    → FunctionBody[]                     │
#   │  Section 11: Data section    → DataSegment[]                      │
#   │  Section 0:  Custom sections → CustomSection[] (any number)       │
#   └────────────────────────────────────────────────────────────────────┘
#
# ─────────────────────────────────────────────────────────────────────────────

module CodingAdventures
  module WasmTypes
    # ─────────────────────────────────────────────────────────────────────────
    # VALUE_TYPE
    # ─────────────────────────────────────────────────────────────────────────

    # VALUE_TYPE — the four numeric types in WASM 1.0.
    #
    # Every local variable, function parameter, function result, and global
    # variable has exactly one of these four types. Each type is encoded as a
    # single byte in the binary format.
    #
    # Binary encoding (each is a single type-code byte):
    #
    #   ┌──────────┬────────┬────────────────────────────────────────────────┐
    #   │ Name     │ Byte   │ Meaning                                        │
    #   ├──────────┼────────┼────────────────────────────────────────────────┤
    #   │ i32      │ 0x7F   │ 32-bit integer (signed or unsigned by opcode)  │
    #   │ i64      │ 0x7E   │ 64-bit integer                                 │
    #   │ f32      │ 0x7D   │ 32-bit IEEE 754 floating-point                 │
    #   │ f64      │ 0x7C   │ 64-bit IEEE 754 floating-point                 │
    #   └──────────┴────────┴────────────────────────────────────────────────┘
    #
    # Note: WASM 1.0 has no boolean type — booleans are represented as i32
    # (0 = false, any nonzero = true).
    #
    # Usage:
    #   CodingAdventures::WasmTypes::VALUE_TYPE[:i32]  # => 0x7F
    VALUE_TYPE = {
      i32: 0x7F, # 32-bit integer
      i64: 0x7E, # 64-bit integer
      f32: 0x7D, # 32-bit IEEE 754 float
      f64: 0x7C  # 64-bit IEEE 754 float
    }.freeze

    # ─────────────────────────────────────────────────────────────────────────
    # BLOCK_TYPE_EMPTY
    # ─────────────────────────────────────────────────────────────────────────

    # BLOCK_TYPE_EMPTY — indicates a structured control block produces no value.
    #
    # In WASM 1.0 a block (block/loop/if) can produce either:
    #   - No result:         encoded as the single byte 0x40
    #   - One value result:  encoded as a ValueType byte
    #
    # The 0x40 byte means the block is a "statement" (produces nothing on the
    # stack) rather than an "expression."
    #
    # Binary layout:
    #   block_instr ::= 0x02  bt:blocktype  (instr)*  0x0B
    #
    # Examples:
    #   [0x02, 0x40, ..., 0x0B]  → block with no result
    #   [0x02, 0x7F, ..., 0x0B]  → block that leaves one i32 on the stack
    BLOCK_TYPE_EMPTY = 0x40

    # ─────────────────────────────────────────────────────────────────────────
    # EXTERNAL_KIND
    # ─────────────────────────────────────────────────────────────────────────

    # EXTERNAL_KIND — classifies what kind of definition is being imported or
    # exported.
    #
    # WASM modules can import and export four kinds of entities:
    #
    #   ┌──────────┬────────┬───────────────────────────────────────────────┐
    #   │ Kind     │ Byte   │ What it refers to                             │
    #   ├──────────┼────────┼───────────────────────────────────────────────┤
    #   │ function │ 0x00   │ A function in the module                      │
    #   │ table    │ 0x01   │ A table (array of function references)        │
    #   │ memory   │ 0x02   │ A linear memory region                       │
    #   │ global   │ 0x03   │ A global variable                            │
    #   └──────────┴────────┴───────────────────────────────────────────────┘
    EXTERNAL_KIND = {
      function: 0x00, # callable function
      table: 0x01,    # table of opaque references
      memory: 0x02,   # linear byte-array memory
      global: 0x03    # single typed value
    }.freeze

    # ─────────────────────────────────────────────────────────────────────────
    # FUNCREF
    # ─────────────────────────────────────────────────────────────────────────

    # FUNCREF — the element type for tables in WASM 1.0.
    #
    # All tables in WASM 1.0 hold function references (funcref = 0x70).
    # The reference-types proposal (WASM 2.0) adds externref (0x6F), but that
    # is outside scope here.
    FUNCREF = 0x70

    # ─────────────────────────────────────────────────────────────────────────
    # FuncType
    # ─────────────────────────────────────────────────────────────────────────

    # FuncType — the type signature of a function.
    #
    # In WASM, every function has a type: a list of parameter types and a list
    # of result types. WASM 1.0 allows at most one result type, but the type
    # section still encodes results as a vector for future compatibility.
    #
    # Binary encoding:
    #
    #   0x60               ← function type prefix byte
    #   [n uleb128]        ← number of params
    #   [param_type]*n     ← each param VALUE_TYPE byte
    #   [m uleb128]        ← number of results
    #   [result_type]*m    ← each result VALUE_TYPE byte
    #
    # Example — function (i32, i64) → f64:
    #
    #   0x60 0x02 0x7F 0x7E 0x01 0x7C
    #         ^^^^ ^^^^ ^^^^ ^^^^ ^^^^
    #         2    i32  i64  1    f64
    #
    # Usage:
    #   ft = FuncType.new([VALUE_TYPE[:i32]], [VALUE_TYPE[:i64]])
    #   ft.params   # => [0x7F]
    #   ft.results  # => [0x7E]
    FuncType = Struct.new(:params, :results) do
      # params  — Array of VALUE_TYPE values (parameter types, left to right)
      # results — Array of VALUE_TYPE values (result types; at most 1 in WASM 1.0)
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Limits
    # ─────────────────────────────────────────────────────────────────────────

    # Limits — describes the size constraints of a memory or table.
    #
    # WASM memories and tables have a minimum size and an optional maximum.
    # Sizes are measured in *pages* for memories (1 page = 64 KiB) and in
    # *elements* for tables.
    #
    # Binary encoding:
    #
    #   0x00 [min uleb128]               ← no maximum (max = nil in Ruby)
    #   0x01 [min uleb128] [max uleb128] ← with maximum
    #
    # Example — memory with at least 1 page, at most 4 pages:
    #   Limits.new(1, 4)
    #
    # Example — memory with at least 2 pages, no maximum:
    #   Limits.new(2, nil)
    Limits = Struct.new(:min, :max) do
      # min — Integer: minimum number of pages (memory) or elements (table)
      # max — Integer or nil: maximum, or nil if unbounded
    end

    # ─────────────────────────────────────────────────────────────────────────
    # MemoryType
    # ─────────────────────────────────────────────────────────────────────────

    # MemoryType — the type of a linear memory region.
    #
    # WebAssembly linear memory is a contiguous, mutable byte array. WASM 1.0
    # allows at most one memory per module. Its size is measured in 64-KiB pages.
    # The maximum addressable memory in WASM 1.0 is 65536 pages = 4 GiB.
    #
    # Binary encoding:
    #   [limits]   ← encoded Limits (see above)
    #
    # Example:
    #   MemoryType.new(Limits.new(1, 16))   # 1–16 pages = 64 KiB – 1 MiB
    MemoryType = Struct.new(:limits) do
      # limits — a Limits instance
    end

    # ─────────────────────────────────────────────────────────────────────────
    # TableType
    # ─────────────────────────────────────────────────────────────────────────

    # TableType — the type of a WASM table.
    #
    # A table is an indexed array of opaque references. In WASM 1.0, the only
    # allowed element type is funcref (0x70). Tables support indirect function
    # calls: call_indirect uses a runtime integer index into the table.
    #
    # Binary encoding:
    #
    #   0x70       ← element type: funcref
    #   [limits]   ← encoded Limits
    #
    # Example:
    #   TableType.new(FUNCREF, Limits.new(10, nil))
    #   → table of at least 10 function references, no maximum
    TableType = Struct.new(:element_type, :limits) do
      # element_type — Integer: always FUNCREF (0x70) in WASM 1.0
      # limits       — a Limits instance
    end

    # ─────────────────────────────────────────────────────────────────────────
    # GlobalType
    # ─────────────────────────────────────────────────────────────────────────

    # GlobalType — the type of a global variable.
    #
    # Globals hold a single value of a VALUE_TYPE. They are either immutable
    # (constants) or mutable (writable from within the module).
    #
    # Binary encoding:
    #   [value_type byte]   ← one of 0x7F/0x7E/0x7D/0x7C
    #   [mutability]        ← 0x00 = immutable, 0x01 = mutable
    #
    # Mutability truth table:
    #   ┌───────────┬────────┬──────────────────────────────────────────────┐
    #   │ mutable   │ byte   │ Allowed operations                           │
    #   ├───────────┼────────┼──────────────────────────────────────────────┤
    #   │  false    │  0x00  │ global.get only                              │
    #   │  true     │  0x01  │ global.get and global.set                    │
    #   └───────────┴────────┴──────────────────────────────────────────────┘
    #
    # Examples:
    #   GlobalType.new(VALUE_TYPE[:i32], false)  # immutable i32 constant
    #   GlobalType.new(VALUE_TYPE[:f64], true)   # mutable f64 variable
    GlobalType = Struct.new(:value_type, :mutable) do
      # value_type — Integer: one of the VALUE_TYPE values
      # mutable    — Boolean: true = global.set allowed, false = read-only
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Import
    # ─────────────────────────────────────────────────────────────────────────

    # Import — a single import declaration.
    #
    # A WASM module can import functions, tables, memories, and globals from
    # the host environment. Each import is identified by two names: the module
    # name (e.g. "env") and the field name (e.g. "memory").
    #
    # Binary encoding:
    #
    #   [mod_len uleb128]     ← byte-length of module name
    #   [mod_bytes...] UTF-8  ← module name
    #   [nm_len uleb128]      ← byte-length of field name
    #   [nm_bytes...] UTF-8   ← field name
    #   [kind byte]           ← EXTERNAL_KIND value (0x00–0x03)
    #   [kind-specific data]  ← depends on kind (see below)
    #
    # Kind-specific type_info:
    #   :function → Integer (type-section index)
    #   :table    → TableType
    #   :memory   → MemoryType
    #   :global   → GlobalType
    #
    # Example:
    #   Import.new("env", "add", :function, 2)
    #     → import function "env"."add" with type at index 2
    Import = Struct.new(:module_name, :name, :kind, :type_info) do
      # module_name — String: the module namespace, e.g. "env"
      # name        — String: the field name within the module
      # kind        — Symbol: one of EXTERNAL_KIND keys (:function, :table, :memory, :global)
      # type_info   — Integer | TableType | MemoryType | GlobalType
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Export
    # ─────────────────────────────────────────────────────────────────────────

    # Export — a single export declaration.
    #
    # Exports make internal definitions (functions, tables, memories, globals)
    # visible to the host environment under a string name.
    #
    # Binary encoding:
    #
    #   [nm_len uleb128]      ← byte-length of export name
    #   [nm_bytes...] UTF-8   ← export name
    #   [kind byte]           ← EXTERNAL_KIND value (0x00–0x03)
    #   [index uleb128]       ← index into the appropriate index space
    #
    # Example:
    #   Export.new("main", :function, 0)
    #     → export function at index 0 as "main"
    Export = Struct.new(:name, :kind, :index) do
      # name  — String: the name visible to the host
      # kind  — Symbol: one of EXTERNAL_KIND keys
      # index — Integer: index into the corresponding index space
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Global
    # ─────────────────────────────────────────────────────────────────────────

    # Global — a module-defined global variable with its initializer.
    #
    # Each global in the global section has a type (GlobalType) and a constant
    # initialization expression. The init expression is a sequence of WASM
    # instructions that produces a single constant value, stored here as a
    # binary String including the trailing `end` opcode (0x0B).
    #
    # In WASM 1.0 the init expression must be a constant expression:
    #   - i32.const N  (opcode 0x41 + signed LEB128)
    #   - i64.const N  (opcode 0x42 + signed LEB128)
    #   - f32.const N  (opcode 0x43 + 4 IEEE754 little-endian bytes)
    #   - f64.const N  (opcode 0x44 + 8 IEEE754 little-endian bytes)
    #   - global.get N (opcode 0x23 — only for imported globals)
    #
    # Binary encoding:
    #   [global_type]      ← GlobalType encoding
    #   [init_expr bytes]  ← constant-expression opcodes + 0x0B (end)
    #
    # Example — immutable i32 global with value 42:
    #   global_type: GlobalType.new(VALUE_TYPE[:i32], false)
    #   init_expr:   "\x41\x2A\x0B".b   (i32.const 42; end)
    Global = Struct.new(:global_type, :init_expr) do
      # global_type — GlobalType
      # init_expr   — String (binary encoding, frozen): constant-expression
      #               bytes including trailing 0x0B (end opcode)
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Element
    # ─────────────────────────────────────────────────────────────────────────

    # Element — a table initializer segment.
    #
    # Element segments populate a table with function references at module
    # instantiation time. This is how call_indirect can call the right function:
    # the table is pre-filled with function indices by element segments.
    #
    # Binary encoding:
    #
    #   [table_index uleb128]    ← which table to fill (always 0 in WASM 1.0)
    #   [offset_expr bytes]      ← constant-expression giving start index
    #   [count uleb128]          ← number of function-index entries
    #   [func_index uleb128]*    ← one entry per function reference
    #
    # Example — fill table 0 starting at slot 5 with functions [0, 1, 2]:
    #   Element.new(0, "\x41\x05\x0B".b, [0, 1, 2])
    Element = Struct.new(:table_index, :offset_expr, :function_indices) do
      # table_index      — Integer: index of the table (always 0 in WASM 1.0)
      # offset_expr      — String (binary): constant-expression for start slot
      # function_indices — Array of Integer: function indices to place in table
    end

    # ─────────────────────────────────────────────────────────────────────────
    # DataSegment
    # ─────────────────────────────────────────────────────────────────────────

    # DataSegment — a linear-memory initializer.
    #
    # Data segments copy bytes into linear memory at instantiation time. This
    # is how compiled programs get read-only string data, lookup tables, and
    # other static data into memory.
    #
    # Binary encoding:
    #
    #   [memory_index uleb128]   ← which memory (always 0 in WASM 1.0)
    #   [offset_expr bytes]      ← constant-expression giving start address
    #   [byte_count uleb128]     ← number of data bytes
    #   [data bytes]*            ← raw byte payload
    #
    # Example — write "Hi" into memory 0 at address 0:
    #   DataSegment.new(0, "\x41\x00\x0B".b, "Hi".b)
    DataSegment = Struct.new(:memory_index, :offset_expr, :data) do
      # memory_index — Integer: index of the memory (always 0 in WASM 1.0)
      # offset_expr  — String (binary): constant-expression for byte address
      # data         — String (binary): raw bytes to copy into memory
    end

    # ─────────────────────────────────────────────────────────────────────────
    # FunctionBody
    # ─────────────────────────────────────────────────────────────────────────

    # FunctionBody — the body of a module-defined function.
    #
    # The code section contains one FunctionBody per module-defined function.
    # Each body has a local-variable declaration list and a raw byte sequence
    # of opcodes.
    #
    # Binary encoding (inside the code section):
    #
    #   [body_size uleb128]              ← total byte length of the body
    #   [local_count uleb128]            ← number of local-decl groups
    #   [local_decl]*                    ← each: [count uleb128] [type byte]
    #   [code bytes...]                  ← instruction sequence
    #   0x0B                             ← end opcode
    #
    # Note: the locals field here stores *expanded* local types (one entry per
    # local), not the run-length-encoded groups in the binary. Decompression
    # is the parser's job.
    #
    # Example — body with two i32 locals and a simple add:
    #   FunctionBody.new(
    #     [VALUE_TYPE[:i32], VALUE_TYPE[:i32]],
    #     "\x20\x00\x20\x01\x6A\x0B".b
    #   )
    #   # locals: [i32, i32]
    #   # code:   local.get 0; local.get 1; i32.add; end
    FunctionBody = Struct.new(:locals, :code) do
      # locals — Array of VALUE_TYPE values (NOT including function parameters)
      # code   — String (binary): raw opcodes including trailing 0x0B (end)
    end

    # ─────────────────────────────────────────────────────────────────────────
    # CustomSection
    # ─────────────────────────────────────────────────────────────────────────

    # CustomSection — an arbitrary named byte blob (section id 0).
    #
    # Custom sections can appear anywhere in a .wasm file and are used for
    # metadata that does not affect module semantics. Common uses:
    #   - "name" section:      debug names for functions, locals, globals
    #   - "producers" section: compiler/tool information
    #   - DWARF debug info
    #
    # Binary encoding:
    #
    #   0x00                         ← section id: custom
    #   [section_size uleb128]       ← total byte size of name + data
    #   [name_len uleb128]           ← byte-length of the name string
    #   [name bytes...] UTF-8        ← name of the custom section
    #   [data bytes...]              ← raw payload (section-specific format)
    #
    # Example:
    #   CustomSection.new("name", "\x00\x04main".b)
    CustomSection = Struct.new(:name, :data) do
      # name — String: section name, e.g. "name" or "producers"
      # data — String (binary): raw payload bytes
    end

    # ─────────────────────────────────────────────────────────────────────────
    # WasmModule
    # ─────────────────────────────────────────────────────────────────────────

    # WasmModule — a mutable in-memory representation of a WASM 1.0 module.
    #
    # This class mirrors the eleven standard sections of the WASM binary format
    # plus zero or more custom sections. A parser reads the binary and populates
    # these arrays; a code generator or validator reads them.
    #
    # Fields are deliberately mutable Arrays so a parser can push entries as it
    # scans sections. Immutability of individual entries is enforced by the
    # Struct types above.
    #
    # Section → field mapping:
    #
    #   ┌─────────────────────┬───────────────────────────────────────────────┐
    #   │ WASM Section        │ WasmModule field                              │
    #   ├─────────────────────┼───────────────────────────────────────────────┤
    #   │ 1  Type             │ types:     Array[FuncType]                    │
    #   │ 2  Import           │ imports:   Array[Import]                      │
    #   │ 3  Function         │ functions: Array[Integer] (type indices)      │
    #   │ 4  Table            │ tables:    Array[TableType]                   │
    #   │ 5  Memory           │ memories:  Array[MemoryType]                  │
    #   │ 6  Global           │ globals:   Array[Global]                      │
    #   │ 7  Export           │ exports:   Array[Export]                      │
    #   │ 8  Start            │ start:     Integer | nil                      │
    #   │ 9  Element          │ elements:  Array[Element]                     │
    #   │ 10 Code             │ code:      Array[FunctionBody]                │
    #   │ 11 Data             │ data:      Array[DataSegment]                 │
    #   │ 0  Custom (many)    │ customs:   Array[CustomSection]               │
    #   └─────────────────────┴───────────────────────────────────────────────┘
    #
    # Usage:
    #   mod = WasmModule.new
    #   mod.types << FuncType.new([VALUE_TYPE[:i32]], [VALUE_TYPE[:i32]])
    #   mod.functions << 0   # function 0 uses type at index 0
    class WasmModule
      # Function type signatures (type section).
      attr_accessor :types

      # Imported definitions from the host (import section).
      attr_accessor :imports

      # Type-section indices for each module-defined function (function section).
      # functions[i] is the index into +types+ for the i-th function body.
      # Note: the "function index space" includes imported functions first.
      attr_accessor :functions

      # Table definitions (table section). WASM 1.0 allows at most one.
      attr_accessor :tables

      # Memory definitions (memory section). WASM 1.0 allows at most one.
      attr_accessor :memories

      # Module-defined globals with their init expressions (global section).
      attr_accessor :globals

      # Exported definitions, visible to the host (export section).
      attr_accessor :exports

      # The index of the start function, or nil if absent (start section).
      # When non-nil, the WASM engine calls this function automatically at
      # instantiation time, before the module is otherwise usable.
      attr_accessor :start

      # Table initializer segments (element section).
      attr_accessor :elements

      # Function bodies in the same order as +functions+ (code section).
      attr_accessor :code

      # Memory initializer segments (data section).
      attr_accessor :data

      # Custom (non-standard) sections, e.g. debug names (custom sections).
      attr_accessor :customs

      def initialize
        @types     = []
        @imports   = []
        @functions = []
        @tables    = []
        @memories  = []
        @globals   = []
        @exports   = []
        @start     = nil
        @elements  = []
        @code      = []
        @data      = []
        @customs   = []
      end
    end
  end
end
