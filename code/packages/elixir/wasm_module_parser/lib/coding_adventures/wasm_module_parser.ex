defmodule CodingAdventures.WasmModuleParser do
  @moduledoc """
  Parse raw `.wasm` binary bytes into a structured `WasmModule`.

  No execution — pure decoding.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## The WebAssembly Binary Format

  A `.wasm` file is a compact binary encoding of a WebAssembly module.
  Every integer uses LEB128 variable-length encoding. Strings are
  length-prefixed UTF-8.

  ```
  ┌──────────────────────────────────────────────────────────────────────────┐
  │  WASM Binary Layout                                                      │
  ├──────────────────────────────────────────────────────────────────────────┤
  │  Magic    │  Version  │  Section  │  Section  │  ...                    │
  │  4 bytes  │  4 bytes  │  id+size+payload      │                         │
  └──────────────────────────────────────────────────────────────────────────┘

  Magic:   <<0x00, 0x61, 0x73, 0x6D>>   (the bytes "\\0asm")
  Version: <<0x01, 0x00, 0x00, 0x00>>   (little-endian u32 = 1)

  Each section:
    ┌──────┬──────────────────┬────────────────────────────────────────┐
    │ id   │ size (u32 leb128)│ payload (size bytes)                   │
    │ 1 B  │ 1–5 bytes        │ contents vary by id                    │
    └──────┴──────────────────┴────────────────────────────────────────┘

  Section IDs:
    0  = Custom     any position, any number of times
    1  = Type       function type signatures
    2  = Import     host-provided imports
    3  = Function   type index for each local function
    4  = Table      indirect-call tables
    5  = Memory     linear memory declarations
    6  = Global     module-level global variables
    7  = Export     names exported to the host
    8  = Start      optional auto-called function
    9  = Element    table initialisation data
   10  = Code       function bodies (locals + bytecode)
   11  = Data       memory initialisation data
  ```

  ## Section Payload Formats

  ```
  Type (§1):
    count: u32leb
    each:  0x60 param_count:u32leb param_types:u8[] result_count:u32leb result_types:u8[]

  Import (§2):
    count: u32leb
    each:  module:str  name:str  kind:u8  type_info
      str = len:u32leb  utf8_bytes
      kind 0 = func   → type_index:u32leb
      kind 1 = table  → element_type:u8  limits
      kind 2 = mem    → limits
      kind 3 = global → valtype:u8  mutable:u8
      limits = flags:u8  min:u32leb  [max:u32leb if flags bit0 set]

  Function (§3):  count:u32leb  type_index:u32leb × count
  Table    (§4):  count:u32leb  element_type:u8  limits × count
  Memory   (§5):  count:u32leb  limits × count

  Global (§6):
    count: u32leb
    each:  valtype:u8  mutable:u8  init_expr (bytes until 0x0B inclusive)

  Export (§7):
    count: u32leb
    each:  name:str  kind:u8  index:u32leb

  Start (§8):  function_index:u32leb

  Element (§9):
    count: u32leb
    each:  table_idx:u32leb  offset_expr  func_count:u32leb  func_idx:u32leb × func_count

  Code (§10):
    count: u32leb
    each:  body_size:u32leb  local_decl_count:u32leb
           (count:u32leb  valtype:u8) × local_decl_count
           code_bytes

  Data (§11):
    count: u32leb
    each:  mem_idx:u32leb  offset_expr  byte_count:u32leb  data:u8 × byte_count

  Custom (§0):  name:str  data:remaining_bytes
  ```

  ## Usage

      iex> bytes = <<0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00>>
      iex> {:ok, m} = CodingAdventures.WasmModuleParser.parse(bytes)
      iex> m.types
      []

  """

  alias CodingAdventures.WasmLeb128

  alias CodingAdventures.WasmTypes.{
    CustomSection,
    DataSegment,
    Element,
    Export,
    FuncType,
    FunctionBody,
    Global,
    GlobalType,
    Import,
    Limits,
    MemoryType,
    TableType,
    WasmModule
  }

  # ── Constants ────────────────────────────────────────────────────────────────

  # The 4-byte magic number at the start of every .wasm file ("\0asm")
  @wasm_magic <<0x00, 0x61, 0x73, 0x6D>>

  # The 4-byte version field (little-endian u32 = 1)
  @wasm_version <<0x01, 0x00, 0x00, 0x00>>

  # Section IDs from the WASM spec
  @section_custom 0
  @section_type 1
  @section_import 2
  @section_function 3
  @section_table 4
  @section_memory 5
  @section_global 6
  @section_export 7
  @section_start 8
  @section_element 9
  @section_code 10
  @section_data 11

  # The byte tag that starts every function type entry in the type section
  @func_type_tag 0x60

  # The `end` opcode terminating constant expressions
  @end_opcode 0x0B

  # ── Public API ────────────────────────────────────────────────────────────────

  @doc """
  Parse a WASM binary into a `WasmModule`.

  Returns `{:ok, %WasmModule{}}` on success, or `{:error, message}` if the
  binary is malformed.

  ## Steps

  1. Validates the 8-byte header (magic `\\0asm` + version 1).
  2. Reads sections one by one; dispatches each to the appropriate section parser.
  3. Returns the first error encountered.

  ## Examples

      iex> bytes = <<0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00>>
      iex> CodingAdventures.WasmModuleParser.parse(bytes)
      {:ok, %CodingAdventures.WasmTypes.WasmModule{}}

      iex> CodingAdventures.WasmModuleParser.parse(<<0xFF, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00>>)
      {:error, "bad magic bytes"}

  """
  @spec parse(binary()) :: {:ok, WasmModule.t()} | {:error, String.t()}
  def parse(data) when is_binary(data) do
    with {:ok, rest} <- parse_header(data),
         {:ok, module} <- parse_sections(rest, %WasmModule{}) do
      {:ok, module}
    end
  end

  # ── Header ────────────────────────────────────────────────────────────────────

  # Validate the 8-byte WASM header.
  #
  # Every .wasm file begins with:
  #   <<0x00, 0x61, 0x73, 0x6D>>  — the bytes "\0asm"
  #   <<0x01, 0x00, 0x00, 0x00>>  — version 1 in little-endian u32
  #
  # Binary pattern matching in Elixir makes this elegant:
  #   <<magic::binary-size(4), version::binary-size(4), rest::binary>>
  #
  defp parse_header(data) when byte_size(data) < 8 do
    {:error,
     "input too short: need at least 8 bytes for the WASM header, got #{byte_size(data)}"}
  end

  defp parse_header(<<magic::binary-size(4), version::binary-size(4), rest::binary>>) do
    cond do
      magic != @wasm_magic ->
        {:error, "bad magic bytes"}

      version != @wasm_version ->
        {:error, "unsupported WASM version: expected version 1"}

      true ->
        {:ok, rest}
    end
  end

  # ── Section loop ─────────────────────────────────────────────────────────────

  # Read sections one by one until the binary is exhausted.
  #
  # Each iteration:
  #   1. Read 1-byte section id
  #   2. Read u32leb size
  #   3. Slice `size` bytes as the payload
  #   4. Dispatch to the appropriate section parser
  #   5. Accumulate results into the module struct
  defp parse_sections(<<>>, module), do: {:ok, module}

  defp parse_sections(<<section_id::8, rest::binary>>, module) do
    with {:ok, {size, rest2}} <- decode_u32leb(rest),
         <<payload::binary-size(size), rest3::binary>> <- rest2 do
      case dispatch_section(section_id, payload, module) do
        {:ok, new_module} -> parse_sections(rest3, new_module)
        {:error, _} = err -> err
      end
    else
      {:error, _} = err -> err
      _ -> {:error, "truncated section payload for section id #{section_id}"}
    end
  end

  defp parse_sections(_, _), do: {:error, "unexpected end of data parsing section"}

  # ── Section dispatcher ────────────────────────────────────────────────────────

  # Route each section ID to the appropriate parser function.
  # Unknown IDs are silently ignored per the WASM forward-compatibility rule.
  defp dispatch_section(@section_custom, payload, module),
    do: parse_custom_section(payload, module)

  defp dispatch_section(@section_type, payload, module),
    do: parse_type_section(payload, module)

  defp dispatch_section(@section_import, payload, module),
    do: parse_import_section(payload, module)

  defp dispatch_section(@section_function, payload, module),
    do: parse_function_section(payload, module)

  defp dispatch_section(@section_table, payload, module),
    do: parse_table_section(payload, module)

  defp dispatch_section(@section_memory, payload, module),
    do: parse_memory_section(payload, module)

  defp dispatch_section(@section_global, payload, module),
    do: parse_global_section(payload, module)

  defp dispatch_section(@section_export, payload, module),
    do: parse_export_section(payload, module)

  defp dispatch_section(@section_start, payload, module),
    do: parse_start_section(payload, module)

  defp dispatch_section(@section_element, payload, module),
    do: parse_element_section(payload, module)

  defp dispatch_section(@section_code, payload, module),
    do: parse_code_section(payload, module)

  defp dispatch_section(@section_data, payload, module),
    do: parse_data_section(payload, module)

  defp dispatch_section(_unknown_id, _payload, module),
    do: {:ok, module}

  # ── Type section (§1) ─────────────────────────────────────────────────────────
  #
  # Parses all function type signatures. Each entry:
  #   0x60  param_count:u32leb  param_types:u8[]  result_count:u32leb  result_types:u8[]
  #
  # These signatures are deduplicated — all other sections reference them by
  # index into this list. E.g., the Function section says "function 0 has type 2"
  # (meaning types[2]).
  defp parse_type_section(payload, module) do
    with {:ok, {count, rest}} <- decode_u32leb(payload),
         {:ok, types, _rest2} <- parse_type_entries(rest, count, []) do
      {:ok, %{module | types: module.types ++ types}}
    end
  end

  defp parse_type_entries(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp parse_type_entries(<<@func_type_tag, rest::binary>>, n, acc) do
    with {:ok, {param_count, rest2}} <- decode_u32leb(rest),
         {:ok, params, rest3} <- read_value_types(rest2, param_count, []),
         {:ok, {result_count, rest4}} <- decode_u32leb(rest3),
         {:ok, results, rest5} <- read_value_types(rest4, result_count, []) do
      entry = %FuncType{params: params, results: results}
      parse_type_entries(rest5, n - 1, [entry | acc])
    end
  end

  defp parse_type_entries(<<tag, _rest::binary>>, _n, _acc),
    do: {:error, "expected function type tag 0x60, got 0x#{Integer.to_string(tag, 16)}"}

  defp parse_type_entries(<<>>, _n, _acc),
    do: {:error, "truncated type section"}

  # ── Import section (§2) ───────────────────────────────────────────────────────
  #
  # Each import entry:
  #   module_name:str  name:str  kind:u8  type_info
  #
  # Imports let the WASM module use functions, tables, memories, and globals
  # provided by the host environment (e.g., the JavaScript runtime).
  defp parse_import_section(payload, module) do
    with {:ok, {count, rest}} <- decode_u32leb(payload),
         {:ok, imports, _rest2} <- parse_import_entries(rest, count, []) do
      {:ok, %{module | imports: module.imports ++ imports}}
    end
  end

  defp parse_import_entries(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp parse_import_entries(rest, n, acc) do
    with {:ok, {module_name, rest2}} <- decode_string(rest),
         {:ok, {name, rest3}} <- decode_string(rest2),
         <<kind_byte::8, rest4::binary>> <- rest3,
         {:ok, {kind, type_info}, rest5} <- decode_import_type(kind_byte, rest4) do
      entry = %Import{module_name: module_name, name: name, kind: kind, type_info: type_info}
      parse_import_entries(rest5, n - 1, [entry | acc])
    else
      {:error, _} = err -> err
      _ -> {:error, "truncated import section"}
    end
  end

  # Decode the kind byte and the type-specific info for one import.
  defp decode_import_type(0x00, rest) do
    with {:ok, {idx, rest2}} <- decode_u32leb(rest) do
      {:ok, {:function, {:function, idx}}, rest2}
    end
  end

  defp decode_import_type(0x01, <<elem_type::8, rest::binary>>) do
    with {:ok, {limits, rest2}} <- decode_limits(rest) do
      tt = %TableType{element_type: elem_type, limits: limits}
      {:ok, {:table, {:table, tt}}, rest2}
    end
  end

  defp decode_import_type(0x02, rest) do
    with {:ok, {limits, rest2}} <- decode_limits(rest) do
      mt = %MemoryType{limits: limits}
      {:ok, {:memory, {:memory, mt}}, rest2}
    end
  end

  defp decode_import_type(0x03, <<vt_byte::8, mut_byte::8, rest::binary>>) do
    with {:ok, vt} <- decode_value_type(vt_byte) do
      gt = %GlobalType{value_type: vt, mutable: mut_byte != 0}
      {:ok, {:global, {:global, gt}}, rest}
    end
  end

  defp decode_import_type(kind, _rest),
    do: {:error, "unknown import kind: 0x#{Integer.to_string(kind, 16)}"}

  # ── Function section (§3) ─────────────────────────────────────────────────────
  #
  # Just a list of type indices, one per locally-defined function.
  # Parallel with the Code section (§10): functions[i] gives the type index,
  # code[i] gives the body.
  defp parse_function_section(payload, module) do
    with {:ok, {count, rest}} <- decode_u32leb(payload),
         {:ok, indices, _rest2} <- read_u32leb_list(rest, count, []) do
      {:ok, %{module | functions: module.functions ++ indices}}
    end
  end

  # ── Table section (§4) ────────────────────────────────────────────────────────
  #
  # Each table entry: element_type:u8  limits
  # In WASM 1.0 element_type is always 0x70 (funcref).
  defp parse_table_section(payload, module) do
    with {:ok, {count, rest}} <- decode_u32leb(payload),
         {:ok, tables, _rest2} <- parse_table_entries(rest, count, []) do
      {:ok, %{module | tables: module.tables ++ tables}}
    end
  end

  defp parse_table_entries(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp parse_table_entries(<<elem_type::8, rest::binary>>, n, acc) do
    with {:ok, {limits, rest2}} <- decode_limits(rest) do
      entry = %TableType{element_type: elem_type, limits: limits}
      parse_table_entries(rest2, n - 1, [entry | acc])
    end
  end

  defp parse_table_entries(<<>>, _n, _acc),
    do: {:error, "truncated table section"}

  # ── Memory section (§5) ───────────────────────────────────────────────────────
  #
  # Each memory entry is just a limits struct (min/max page counts).
  # 1 page = 64 KiB = 65,536 bytes.
  defp parse_memory_section(payload, module) do
    with {:ok, {count, rest}} <- decode_u32leb(payload),
         {:ok, memories, _rest2} <- parse_memory_entries(rest, count, []) do
      {:ok, %{module | memories: module.memories ++ memories}}
    end
  end

  defp parse_memory_entries(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp parse_memory_entries(rest, n, acc) do
    with {:ok, {limits, rest2}} <- decode_limits(rest) do
      entry = %MemoryType{limits: limits}
      parse_memory_entries(rest2, n - 1, [entry | acc])
    end
  end

  # ── Global section (§6) ───────────────────────────────────────────────────────
  #
  # Each entry: valtype:u8  mutable:u8  init_expr
  # init_expr is a constant-expression byte sequence ending with 0x0B (end).
  defp parse_global_section(payload, module) do
    with {:ok, {count, rest}} <- decode_u32leb(payload),
         {:ok, globals, _rest2} <- parse_global_entries(rest, count, []) do
      {:ok, %{module | globals: module.globals ++ globals}}
    end
  end

  defp parse_global_entries(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp parse_global_entries(<<vt_byte::8, mut_byte::8, rest::binary>>, n, acc) do
    with {:ok, vt} <- decode_value_type(vt_byte),
         {:ok, {init_expr_list, rest2}} <- read_expr(rest) do
      gt = %GlobalType{value_type: vt, mutable: mut_byte != 0}
      entry = %Global{global_type: gt, init_expr: :erlang.list_to_binary(init_expr_list)}
      parse_global_entries(rest2, n - 1, [entry | acc])
    end
  end

  defp parse_global_entries(<<>>, _n, _acc),
    do: {:error, "truncated global section"}

  # ── Export section (§7) ───────────────────────────────────────────────────────
  #
  # Each entry: name:str  kind:u8  index:u32leb
  # Exports make internal things (functions, tables, memories, globals) visible
  # to the host environment under a human-readable name.
  defp parse_export_section(payload, module) do
    with {:ok, {count, rest}} <- decode_u32leb(payload),
         {:ok, exports, _rest2} <- parse_export_entries(rest, count, []) do
      {:ok, %{module | exports: module.exports ++ exports}}
    end
  end

  defp parse_export_entries(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp parse_export_entries(rest, n, acc) do
    with {:ok, {name, rest2}} <- decode_string(rest),
         <<kind_byte::8, rest3::binary>> <- rest2,
         {:ok, kind} <- decode_external_kind(kind_byte),
         {:ok, {idx, rest4}} <- decode_u32leb(rest3) do
      entry = %Export{name: name, kind: kind, index: idx}
      parse_export_entries(rest4, n - 1, [entry | acc])
    else
      {:error, _} = err -> err
      _ -> {:error, "truncated export section"}
    end
  end

  # ── Start section (§8) ────────────────────────────────────────────────────────
  #
  # Just a single function index. The runtime calls this function automatically
  # at instantiation time (after memory/table initialization).
  defp parse_start_section(payload, module) do
    with {:ok, {idx, _rest}} <- decode_u32leb(payload) do
      {:ok, %{module | start: idx}}
    end
  end

  # ── Element section (§9) ──────────────────────────────────────────────────────
  #
  # Initialises table entries at instantiation time.
  # Entry: table_idx:u32leb  offset_expr  func_count:u32leb  func_idx[]
  defp parse_element_section(payload, module) do
    with {:ok, {count, rest}} <- decode_u32leb(payload),
         {:ok, elements, _rest2} <- parse_element_entries(rest, count, []) do
      {:ok, %{module | elements: module.elements ++ elements}}
    end
  end

  defp parse_element_entries(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp parse_element_entries(rest, n, acc) do
    with {:ok, {table_index, rest2}} <- decode_u32leb(rest),
         {:ok, {offset_expr, rest3}} <- read_expr(rest2),
         {:ok, {func_count, rest4}} <- decode_u32leb(rest3),
         {:ok, func_indices, rest5} <- read_u32leb_list(rest4, func_count, []) do
      entry = %Element{
        table_index: table_index,
        offset_expr: :erlang.list_to_binary(offset_expr),
        function_indices: func_indices
      }
      parse_element_entries(rest5, n - 1, [entry | acc])
    end
  end

  # ── Code section (§10) ────────────────────────────────────────────────────────
  #
  # Each function body:
  #   body_size:u32leb
  #   local_decl_count:u32leb
  #   (count:u32leb  valtype:u8) × local_decl_count   ← run-length encoded
  #   code_bytes (remainder, including trailing 0x0B)
  #
  # We expand the run-length-encoded local declarations into a flat list:
  # "3 × i32, 2 × f64" becomes [:i32, :i32, :i32, :f64, :f64].
  defp parse_code_section(payload, module) do
    with {:ok, {count, rest}} <- decode_u32leb(payload),
         {:ok, bodies, _rest2} <- parse_code_entries(rest, count, []) do
      {:ok, %{module | code: module.code ++ bodies}}
    end
  end

  defp parse_code_entries(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp parse_code_entries(rest, n, acc) do
    with {:ok, {body_size, rest2}} <- decode_u32leb(rest),
         <<body_bytes::binary-size(body_size), rest3::binary>> <- rest2,
         {:ok, {local_decl_count, body_rest}} <- decode_u32leb(body_bytes),
         {:ok, locals, body_rest2} <- expand_local_decls(body_rest, local_decl_count, []) do
      # Everything left in the body is the code (including trailing 0x0B)
      code = body_rest2
      entry = %FunctionBody{locals: locals, code: code}
      parse_code_entries(rest3, n - 1, [entry | acc])
    else
      {:error, _} = err -> err
      _ -> {:error, "truncated code section"}
    end
  end

  # Expand run-length-encoded local declarations into a flat list of value types.
  # Binary: (count:u32leb  valtype:u8) × local_decl_count
  defp expand_local_decls(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp expand_local_decls(rest, n, acc) do
    with {:ok, {count, rest2}} <- decode_u32leb(rest),
         <<vt_byte::8, rest3::binary>> <- rest2,
         {:ok, vt} <- decode_value_type(vt_byte) do
      new_locals = List.duplicate(vt, count)
      expand_local_decls(rest3, n - 1, Enum.reverse(new_locals) ++ acc)
    else
      {:error, _} = err -> err
      _ -> {:error, "truncated local declarations"}
    end
  end

  # ── Data section (§11) ────────────────────────────────────────────────────────
  #
  # Initialises linear memory at instantiation time.
  # Entry: mem_idx:u32leb  offset_expr  byte_count:u32leb  data:u8[]
  defp parse_data_section(payload, module) do
    with {:ok, {count, rest}} <- decode_u32leb(payload),
         {:ok, segments, _rest2} <- parse_data_entries(rest, count, []) do
      {:ok, %{module | data: module.data ++ segments}}
    end
  end

  defp parse_data_entries(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp parse_data_entries(rest, n, acc) do
    with {:ok, {mem_idx, rest2}} <- decode_u32leb(rest),
         {:ok, {offset_expr, rest3}} <- read_expr(rest2),
         {:ok, {byte_count, rest4}} <- decode_u32leb(rest3),
         <<data::binary-size(byte_count), rest5::binary>> <- rest4 do
      entry = %DataSegment{
        memory_index: mem_idx,
        offset_expr: :erlang.list_to_binary(offset_expr),
        data: data
      }
      parse_data_entries(rest5, n - 1, [entry | acc])
    else
      {:error, _} = err -> err
      _ -> {:error, "truncated data section"}
    end
  end

  # ── Custom section (§0) ───────────────────────────────────────────────────────
  #
  # name:str  data:remaining_bytes
  # Custom sections carry tooling metadata (debug names, source maps, DWARF, etc.)
  # and are ignored by the WASM runtime.
  defp parse_custom_section(payload, module) do
    with {:ok, {name, rest}} <- decode_string(payload) do
      entry = %CustomSection{name: name, data: rest}
      {:ok, %{module | customs: module.customs ++ [entry]}}
    end
  end

  # ── Primitive decoders ────────────────────────────────────────────────────────

  # Decode a u32 LEB128 integer from the head of a binary.
  # Returns {:ok, {value, rest}} where rest is the unconsumed tail.
  #
  # We use the wasm_leb128 library, which works on offset-based slices.
  # By passing offset 0 we read from the start of the remaining binary.
  defp decode_u32leb(<<>>) do
    {:error, "unexpected end of data decoding LEB128"}
  end

  defp decode_u32leb(data) do
    case WasmLeb128.decode_unsigned(data, 0) do
      {:ok, {val, consumed}} ->
        <<_::binary-size(consumed), rest::binary>> = data
        {:ok, {val, rest}}

      {:error, msg} ->
        {:error, msg}
    end
  end

  # Decode a length-prefixed UTF-8 string.
  # Format: len:u32leb  utf8_bytes × len
  defp decode_string(data) do
    with {:ok, {len, rest}} <- decode_u32leb(data),
         <<str_bytes::binary-size(len), rest2::binary>> <- rest do
      case :unicode.characters_to_binary(str_bytes, :utf8, :utf8) do
        str when is_binary(str) -> {:ok, {str, rest2}}
        _ -> {:error, "invalid UTF-8 in string"}
      end
    else
      {:error, _} = err -> err
      _ -> {:error, "truncated string"}
    end
  end

  # Decode a limits struct (used by tables and memories).
  # Format: flags:u8  min:u32leb  [max:u32leb if flags bit0 set]
  defp decode_limits(<<flags::8, rest::binary>>) do
    with {:ok, {min_val, rest2}} <- decode_u32leb(rest) do
      if Bitwise.band(flags, 0x01) != 0 do
        with {:ok, {max_val, rest3}} <- decode_u32leb(rest2) do
          {:ok, {%Limits{min: min_val, max: max_val}, rest3}}
        end
      else
        {:ok, {%Limits{min: min_val, max: nil}, rest2}}
      end
    end
  end

  defp decode_limits(<<>>), do: {:error, "truncated limits"}

  # Decode a value type byte into an atom.
  #
  # ```
  # 0x7F → :i32
  # 0x7E → :i64
  # 0x7D → :f32
  # 0x7C → :f64
  # ```
  defp decode_value_type(0x7F), do: {:ok, :i32}
  defp decode_value_type(0x7E), do: {:ok, :i64}
  defp decode_value_type(0x7D), do: {:ok, :f32}
  defp decode_value_type(0x7C), do: {:ok, :f64}

  defp decode_value_type(b),
    do: {:error, "unknown value type byte: 0x#{Integer.to_string(b, 16)}"}

  # Decode an external kind byte into an atom.
  defp decode_external_kind(0x00), do: {:ok, :function}
  defp decode_external_kind(0x01), do: {:ok, :table}
  defp decode_external_kind(0x02), do: {:ok, :memory}
  defp decode_external_kind(0x03), do: {:ok, :global}

  defp decode_external_kind(b),
    do: {:error, "unknown external kind: 0x#{Integer.to_string(b, 16)}"}

  # Read `n` value type bytes into a list.
  defp read_value_types(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp read_value_types(<<b::8, rest::binary>>, n, acc) do
    with {:ok, vt} <- decode_value_type(b) do
      read_value_types(rest, n - 1, [vt | acc])
    end
  end

  defp read_value_types(<<>>, _n, _acc),
    do: {:error, "truncated value type list"}

  # Read `n` u32leb values into a list.
  defp read_u32leb_list(rest, 0, acc), do: {:ok, Enum.reverse(acc), rest}

  defp read_u32leb_list(rest, n, acc) do
    with {:ok, {val, rest2}} <- decode_u32leb(rest) do
      read_u32leb_list(rest2, n - 1, [val | acc])
    end
  end

  # Read a constant expression (init_expr, offset_expr): bytes until and
  # including the end opcode (0x0B).
  #
  # Constant expressions are restricted by the WASM spec to:
  #   i32.const / i64.const / f32.const / f64.const / global.get / end
  #
  # We parse the immediates for the known opcodes and pass through unknown ones
  # for forward-compatibility.
  defp read_expr(data), do: read_expr_loop(data, [])

  defp read_expr_loop(<<@end_opcode, rest::binary>>, acc) do
    {:ok, {Enum.reverse([@end_opcode | acc]), rest}}
  end

  defp read_expr_loop(<<>>, _acc), do: {:error, "unterminated constant expression"}

  # i32.const — LEB128 immediate
  defp read_expr_loop(<<0x41, rest::binary>>, acc) do
    with {:ok, {_val, consumed}} <- WasmLeb128.decode_unsigned(rest, 0) do
      <<imm::binary-size(consumed), rest2::binary>> = rest
      imm_bytes = :erlang.binary_to_list(imm)
      read_expr_loop(rest2, Enum.reverse(imm_bytes) ++ [0x41 | acc])
    end
  end

  # i64.const — LEB128 immediate
  defp read_expr_loop(<<0x42, rest::binary>>, acc) do
    with {:ok, {_val, consumed}} <- WasmLeb128.decode_unsigned(rest, 0) do
      <<imm::binary-size(consumed), rest2::binary>> = rest
      imm_bytes = :erlang.binary_to_list(imm)
      read_expr_loop(rest2, Enum.reverse(imm_bytes) ++ [0x42 | acc])
    end
  end

  # f32.const — 4 raw bytes
  defp read_expr_loop(<<0x43, b0::8, b1::8, b2::8, b3::8, rest::binary>>, acc) do
    read_expr_loop(rest, [b3, b2, b1, b0, 0x43 | acc])
  end

  # f64.const — 8 raw bytes
  defp read_expr_loop(
         <<0x44, b0::8, b1::8, b2::8, b3::8, b4::8, b5::8, b6::8, b7::8, rest::binary>>,
         acc
       ) do
    read_expr_loop(rest, [b7, b6, b5, b4, b3, b2, b1, b0, 0x44 | acc])
  end

  # global.get — LEB128 immediate
  defp read_expr_loop(<<0x23, rest::binary>>, acc) do
    with {:ok, {_val, consumed}} <- WasmLeb128.decode_unsigned(rest, 0) do
      <<imm::binary-size(consumed), rest2::binary>> = rest
      imm_bytes = :erlang.binary_to_list(imm)
      read_expr_loop(rest2, Enum.reverse(imm_bytes) ++ [0x23 | acc])
    end
  end

  # Unknown opcode — add it and keep scanning for end
  defp read_expr_loop(<<b::8, rest::binary>>, acc) do
    read_expr_loop(rest, [b | acc])
  end
end
