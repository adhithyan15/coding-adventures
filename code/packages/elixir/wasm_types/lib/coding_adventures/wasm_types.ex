defmodule CodingAdventures.WasmTypes do
  @moduledoc """
  Pure type definitions for the WebAssembly 1.0 (MVP) type system.

  This module contains no parsing logic. It provides byte constants,
  constructor helpers, and struct definitions that represent a decoded
  WASM module's type information. Higher-level modules like `wasm-opcodes`
  and `wasm-module-parser` depend on these definitions.

  ## Where types live in the WASM binary format

  A `.wasm` file is a sequence of **sections**. Each section has an ID byte,
  a byte-length, then its contents:

  ```
  .wasm file layout
  ┌──────────────────────────────────────────────────────────┐
  │ Magic: 0x00 0x61 0x73 0x6D  ("asm")                     │
  │ Version: 0x01 0x00 0x00 0x00                             │
  ├──────┬────────────────────────────────────────────────── │
  │ §  1 │ Type section   → list of FuncType                 │
  │ §  2 │ Import section → list of Import                   │
  │ §  3 │ Function section → list of u32 (type indices)     │
  │ §  4 │ Table section  → list of TableType                │
  │ §  5 │ Memory section → list of MemoryType               │
  │ §  6 │ Global section → list of Global                   │
  │ §  7 │ Export section → list of Export                   │
  │ §  8 │ Start section  → optional u32                     │
  │ §  9 │ Element section → list of Element                 │
  │ § 10 │ Code section   → list of FunctionBody             │
  │ § 11 │ Data section   → list of DataSegment              │
  │ §  0 │ Custom sections (name, debug info, etc.)           │
  └──────┴────────────────────────────────────────────────────
  ```

  ## Elixir structs and "mutability"

  In Elixir, all data is immutable — there is no mutable state. Structs are
  just maps with a `__struct__` key. "Updating" a struct means creating a new
  struct with updated fields using the `%{struct | field: value}` syntax.
  This is the idiomatic Elixir way and matches WASM's functional semantics.
  """

  # ────────────────────────────────────────────────────────────────────────────
  # ValueType byte constants
  # ────────────────────────────────────────────────────────────────────────────

  # The four numeric value types that WASM 1.0 supports.
  #
  # Byte encoding in the WASM binary (signed LEB128, counting down from 0x7F):
  #
  #   ┌────────┬──────┐
  #   │  Type  │ Byte │
  #   ├────────┼──────┤
  #   │  i32   │ 0x7F │
  #   │  i64   │ 0x7E │
  #   │  f32   │ 0x7D │
  #   │  f64   │ 0x7C │
  #   └────────┴──────┘

  @i32 0x7F
  @i64 0x7E
  @f32 0x7D
  @f64 0x7C

  @doc """
  Returns the WASM binary byte tag for a value type.

  ## Value types

  WASM 1.0 has four numeric value types. Every local variable, function
  parameter, return value, and stack slot holds one of these:

  - `:i32` — 32-bit integer. Also used for booleans and linear-memory pointers.
  - `:i64` — 64-bit integer.
  - `:f32` — 32-bit IEEE 754 single-precision float.
  - `:f64` — 64-bit IEEE 754 double-precision float.

  ## Examples

      iex> CodingAdventures.WasmTypes.value_type(:i32)
      0x7F

      iex> CodingAdventures.WasmTypes.value_type(:f64)
      0x7C

  """
  def value_type(:i32), do: @i32
  def value_type(:i64), do: @i64
  def value_type(:f32), do: @f32
  def value_type(:f64), do: @f64

  @doc """
  Returns the WASM binary byte tag for an empty block type (0x40).

  A block with no result type is encoded as 0x40. This is -64 in signed
  LEB128, which does not overlap with any ValueType byte (0x7C–0x7F).

  ## Example

      iex> CodingAdventures.WasmTypes.block_type_empty()
      0x40

  """
  def block_type_empty(), do: 0x40

  @doc """
  Returns the WASM binary byte tag for an external import/export kind.

  ## Kinds

  ```
  ┌──────────┬──────┐
  │  Kind    │ Byte │
  ├──────────┼──────┤
  │ function │ 0x00 │
  │ table    │ 0x01 │
  │ memory   │ 0x02 │
  │ global   │ 0x03 │
  └──────────┴──────┘
  ```

  ## Examples

      iex> CodingAdventures.WasmTypes.external_kind(:function)
      0x00

      iex> CodingAdventures.WasmTypes.external_kind(:global)
      0x03

  """
  def external_kind(:function), do: 0x00
  def external_kind(:table),    do: 0x01
  def external_kind(:memory),   do: 0x02
  def external_kind(:global),   do: 0x03
end

# ──────────────────────────────────────────────────────────────────────────────
# FuncType
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.FuncType do
  @moduledoc """
  A function's type signature: parameter types and result types.

  WASM 1.0 allows at most one result type in practice (though the binary
  format supports vectors). All function signatures live in the **type
  section** and are referenced by index elsewhere.

  ```
  Binary encoding (type section entry):
    0x60                  ;; function type tag
    <num_params: LEB128>
    <param_type>*         ;; one byte per param (ValueType encoding)
    <num_results: LEB128>
    <result_type>*

  Example: (i32, i64) -> f32
    0x60  02  7F 7E  01  7D
  ```

  ## Fields

  - `params` — list of atom ValueType names (`:i32`, `:i64`, `:f32`, `:f64`)
  - `results` — list of atom ValueType names

  ## Example

      iex> %CodingAdventures.WasmTypes.FuncType{params: [:i32], results: [:i64]}
      %CodingAdventures.WasmTypes.FuncType{params: [:i32], results: [:i64]}

  """

  defstruct params: [], results: []
end

# ──────────────────────────────────────────────────────────────────────────────
# Limits
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.Limits do
  @moduledoc """
  Size constraints (min and optional max) for a memory or table.

  For memories, sizes are in **pages** (1 page = 64 KiB = 65,536 bytes).
  For tables, sizes are in **entries** (number of function references).

  ```
  Binary encoding:
    0x00  <min: LEB128>                    ;; no maximum
    0x01  <min: LEB128>  <max: LEB128>     ;; with maximum

  Example: at least 1 page, at most 4 pages
    0x01  01  04
  ```

  ## Fields

  - `min` — minimum size (required, always present)
  - `max` — maximum size (`nil` means unbounded)

  ## Example

      iex> %CodingAdventures.WasmTypes.Limits{min: 1, max: nil}
      %CodingAdventures.WasmTypes.Limits{min: 1, max: nil}

  """

  defstruct min: 0, max: nil
end

# ──────────────────────────────────────────────────────────────────────────────
# MemoryType
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.MemoryType do
  @moduledoc """
  The type of a linear memory — just its size limits.

  WASM 1.0 allows at most one memory per module. A memory is a contiguous
  byte array that can be read and written by both the module and the host.
  It can grow at runtime (via `memory.grow`) up to `limits.max`.

  ```
  Host (JavaScript)                    WASM module
  ┌──────────────────────────────────────────────────────┐
  │ new WebAssembly.Memory({initial:1, maximum:4})       │
  └──────────────────────────────────────────────────────┘
       limits = %Limits{min: 1, max: 4}
  ```

  ## Fields

  - `limits` — a `Limits` struct with min/max page counts

  ## Example

      iex> %CodingAdventures.WasmTypes.MemoryType{
      ...>   limits: %CodingAdventures.WasmTypes.Limits{min: 1, max: nil}
      ...> }

  """

  defstruct limits: nil
end

# ──────────────────────────────────────────────────────────────────────────────
# TableType
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.TableType do
  @moduledoc """
  The type of a WASM table: an array of references with size limits.

  WASM 1.0 tables hold function references (`funcref`, byte value 0x70).
  Tables are used by the `call_indirect` instruction for indirect calls
  (equivalent to C function pointers or vtables).

  ```
  Table layout (conceptually):

    index:    0         1         2         3
            ┌─────────┬─────────┬─────────┬─────────┐
            │ func #5 │  null   │ func #2 │ func #7 │  ...
            └─────────┴─────────┴─────────┴─────────┘

  call_indirect type_idx
    → pops i32 index, looks up function ref, validates type, calls it
  ```

  ## Fields

  - `element_type` — byte tag for the reference type; always `0x70` (funcref) in WASM 1.0
  - `limits` — a `Limits` struct with min/max entry counts

  ## Example

      iex> %CodingAdventures.WasmTypes.TableType{element_type: 0x70,
      ...>   limits: %CodingAdventures.WasmTypes.Limits{min: 0, max: nil}}

  """

  # 0x70 = funcref — the only reference type in WASM 1.0
  defstruct element_type: 0x70, limits: nil
end

# ──────────────────────────────────────────────────────────────────────────────
# GlobalType
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.GlobalType do
  @moduledoc """
  The type of a global variable: its value type and mutability.

  Immutable globals are constants (e.g., the base address of a data segment).
  Mutable globals hold changing state (e.g., a shadow stack pointer).

  ```
  Binary encoding:
    <value_type: byte>  <mutability: 0x00 or 0x01>

  Example: mutable i32
    7F 01
  ```

  ## Fields

  - `value_type` — atom (`:i32`, `:i64`, `:f32`, or `:f64`)
  - `mutable` — boolean; `true` means the global can be modified

  ## Example

      iex> %CodingAdventures.WasmTypes.GlobalType{value_type: :i32, mutable: true}

  """

  defstruct value_type: nil, mutable: false
end

# ──────────────────────────────────────────────────────────────────────────────
# Import
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.Import do
  @moduledoc """
  A single import declaration from the import section.

  Every import names the *module* that provides it (e.g., `"env"`,
  `"wasi_snapshot_preview1"`) and the *name* within that module
  (e.g., `"memory"`, `"fd_write"`).

  ```
  Binary encoding:
    <module_name: length-prefixed UTF-8>
    <name: length-prefixed UTF-8>
    <kind: ExternalKind byte>
    <type_info: varies by kind>

  Example: import function "env"."abort" with type index 0
    03 "env"  05 "abort"  00  00
  ```

  ## Fields

  - `module_name` — the import namespace string (e.g., `"env"`)
  - `name` — the name within the namespace (e.g., `"abort"`)
  - `kind` — atom: `:function`, `:table`, `:memory`, or `:global`
  - `type_info` — one of:
    - `{:function, type_index}` — index into the type section
    - `{:table, %TableType{}}` — table type
    - `{:memory, %MemoryType{}}` — memory type
    - `{:global, %GlobalType{}}` — global type

  """

  defstruct module_name: "", name: "", kind: nil, type_info: nil
end

# ──────────────────────────────────────────────────────────────────────────────
# Export
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.Export do
  @moduledoc """
  A single export declaration from the export section.

  Exports make module-internal entities visible to the host. For example,
  a compiled C program might export its `main` function and its heap memory.

  ```
  Binary encoding:
    <name: length-prefixed UTF-8>
    <kind: ExternalKind byte>
    <index: LEB128>   ;; index into the appropriate index space

  Example: export function 3 as "main"
    04 "main"  00  03
  ```

  ## Fields

  - `name` — the exported name (e.g., `"main"`, `"memory"`)
  - `kind` — atom: `:function`, `:table`, `:memory`, or `:global`
  - `index` — index into the relevant index space

  """

  defstruct name: "", kind: nil, index: 0
end

# ──────────────────────────────────────────────────────────────────────────────
# Global (module-defined global variable)
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.Global do
  @moduledoc """
  A module-defined global variable with its initialization expression.

  Global variables are initialized by a *constant expression* — a short
  byte sequence of WASM instructions ending with the `end` opcode (0x0B).
  The expression must be computable at instantiation time.

  ```
  Example: `(global i32 (i32.const 42))`
    global_type: %GlobalType{value_type: :i32, mutable: false}
    init_expr:   <<0x41, 0x2A, 0x0B>>
                  │      │     └── end opcode
                  │      └──────── LEB128(42) = 0x2A
                  └────────────── i32.const opcode
  ```

  ## Fields

  - `global_type` — a `GlobalType` struct
  - `init_expr` — binary (raw bytes of the constant initializer, including `end`)

  """

  defstruct global_type: nil, init_expr: <<>>
end

# ──────────────────────────────────────────────────────────────────────────────
# Element
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.Element do
  @moduledoc """
  An element segment — initializes a range of table entries with function indices.

  Element segments populate function tables at module instantiation time.
  The runtime copies `function_indices` into the table at position
  computed by `offset_expr`.

  ```
  Conceptually:
    table[offset_expr()] = [func_0, func_1, func_2, ...]

  Use case: C/C++ function pointer tables, C++ vtables, dynamic dispatch.
  ```

  ## Fields

  - `table_index` — which table to initialize (always 0 in WASM 1.0)
  - `offset_expr` — binary: constant expression for starting offset
  - `function_indices` — list of integers (function indices)

  """

  defstruct table_index: 0, offset_expr: <<>>, function_indices: []
end

# ──────────────────────────────────────────────────────────────────────────────
# DataSegment
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.DataSegment do
  @moduledoc """
  A data segment — initializes a region of linear memory with static bytes.

  Data segments load static data (string literals, lookup tables, initialized
  globals) into WASM memory at instantiation time.

  ```
  Conceptually:
    memory[offset_expr()] = data

  Example: store "hello" at byte 1024
    memory_index: 0
    offset_expr:  <<0x41, 0x80, 0x08, 0x0B>>   ;; i32.const 1024; end
    data:         "hello"
  ```

  ## Fields

  - `memory_index` — which memory to write into (always 0 in WASM 1.0)
  - `offset_expr` — binary: constant expression for byte offset
  - `data` — binary: the bytes to copy into memory

  """

  defstruct memory_index: 0, offset_expr: <<>>, data: <<>>
end

# ──────────────────────────────────────────────────────────────────────────────
# FunctionBody
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.FunctionBody do
  @moduledoc """
  The body of a locally-defined function: its local variable types and bytecode.

  In the WASM binary, locals are declared compactly (run-length encoded).
  This struct stores them as a flat list — one atom per local slot — for
  convenient access. Parameters are NOT included here; they appear in `FuncType`.

  ```
  Binary structure (code section entry):
    <body_size: LEB128>
    <num_local_decls: LEB128>
    (<count: LEB128>  <type: byte>)*   ;; run-length encoded locals
    <instructions...>
    0x0B                               ;; end opcode

  Example: function with 2 i32 locals and code [i32.const 1, end]
    locals: [:i32, :i32]
    code:   <<0x41, 0x01, 0x0B>>
  ```

  ## Fields

  - `locals` — list of value type atoms (expanded, not run-length encoded)
  - `code` — binary: raw instruction bytes (including trailing `end` 0x0B)

  """

  defstruct locals: [], code: <<>>
end

# ──────────────────────────────────────────────────────────────────────────────
# CustomSection
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.CustomSection do
  @moduledoc """
  A custom section — arbitrary named data embedded in a WASM file.

  Custom sections (section ID 0) are ignored by the WASM runtime but carry
  metadata for tooling:

  - `"name"` — maps function indices to human-readable names (debuggers)
  - `"sourceMappingURL"` — points to a source map file
  - DWARF debug sections (used by Rust's WASM target, wasm-pack, etc.)

  ```
  Binary encoding:
    0x00                            ;; section ID = custom
    <section_size: LEB128>
    <name: length-prefixed UTF-8>   ;; name of this custom section
    <data: bytes>                   ;; arbitrary payload
  ```

  ## Fields

  - `name` — the section name (e.g., `"name"`, `"sourceMappingURL"`)
  - `data` — binary: the raw payload

  """

  defstruct name: "", data: <<>>
end

# ──────────────────────────────────────────────────────────────────────────────
# WasmModule
# ──────────────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.WasmTypes.WasmModule do
  @moduledoc """
  A fully decoded WebAssembly 1.0 module.

  This struct holds all data from all sections of a `.wasm` file after parsing.
  It is the intermediate representation between the raw binary and higher-level
  analysis (validation, interpretation, compilation).

  ```
  Relationship between fields:

    types[i]      ←── functions[j]  (functions[j] is a type index)
                  ←── imports with type_info {:function, i}
                  ←── block type_index references

    functions[j]  ←── code[j - num_imported_funcs]  (function body)

    tables[0]     ←── elements[k].table_index

    memories[0]   ←── data[k].memory_index
  ```

  The default is an empty module (all fields empty/nil), which is the natural
  starting state for an incremental parser.

  ## Fields

  - `types`     — list of `FuncType` (type section §1)
  - `imports`   — list of `Import` (import section §2)
  - `functions` — list of integers (function section §3; type indices)
  - `tables`    — list of `TableType` (table section §4)
  - `memories`  — list of `MemoryType` (memory section §5)
  - `globals`   — list of `Global` (global section §6)
  - `exports`   — list of `Export` (export section §7)
  - `start`     — optional integer (start section §8)
  - `elements`  — list of `Element` (element section §9)
  - `code`      — list of `FunctionBody` (code section §10)
  - `data`      — list of `DataSegment` (data section §11)
  - `customs`   — list of `CustomSection` (custom sections §0)

  """

  defstruct types: [],
            imports: [],
            functions: [],
            tables: [],
            memories: [],
            globals: [],
            exports: [],
            start: nil,
            elements: [],
            code: [],
            data: [],
            customs: []
end
