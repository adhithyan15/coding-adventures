# frozen_string_literal: true

# test_wasm_module_parser.rb — Comprehensive tests for WasmModuleParser
#
# Tests cover:
#   1.  Minimal module (header only)
#   2.  Type section: (i32, i32) → i32
#   3.  Function section: type indices
#   4.  Export section: function export
#   5.  Code section: function with locals and instructions
#   6.  Import section: function import
#   7.  Memory section
#   8.  Table section
#   9.  Global section (immutable i32)
#   10. Data section
#   11. Element section
#   12. Start section
#   13. Custom section (name + data)
#   14. Multi-section module
#   15. Error: bad magic bytes
#   16. Error: wrong version
#   17. Error: truncated header
#   18. Error: truncated section payload
#   19. Round-trip test: build binary, parse, verify all fields
#
# ─────────────────────────────────────────────────────────────────────────────
# TEST HELPER UTILITIES
# ─────────────────────────────────────────────────────────────────────────────
#
# We build test .wasm binaries by hand to avoid any dependency on a real
# compiler. This also documents the binary format precisely.

require "minitest/autorun"
require "coding_adventures_wasm_module_parser"

class TestWasmModuleParser < Minitest::Test
  # ── Helper methods ───────────────────────────────────────────────────────

  # encode_uleb128 — encode a non-negative integer as ULEB128 (array of bytes).
  #
  # Algorithm:
  #   do { byte = value & 0x7F; value >>= 7; byte |= 0x80 if value > 0; emit }
  #   while value > 0
  def encode_uleb128(n)
    bytes = []
    remaining = n & 0xFFFFFFFF  # treat as u32
    loop do
      byte = remaining & 0x7F
      remaining >>= 7
      byte |= 0x80 if remaining != 0
      bytes << byte
      break if remaining == 0
    end
    bytes
  end

  # make_section — build a WASM section: id:u8 + size:u32leb + payload.
  def make_section(id, payload)
    size_bytes = encode_uleb128(payload.length)
    [id, *size_bytes, *payload]
  end

  # make_string — encode a WASM name string: length:u32leb + UTF-8 bytes.
  def make_string(s)
    bytes = s.bytes
    [*encode_uleb128(bytes.length), *bytes]
  end

  # WASM_HEADER — the 8-byte header that starts every valid .wasm file.
  #
  #   Magic:   \0asm = [0x00, 0x61, 0x73, 0x6D]
  #   Version: 1     = [0x01, 0x00, 0x00, 0x00] (little-endian uint32)
  WASM_HEADER = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00].freeze

  # make_wasm — combine WASM_HEADER + sections into a complete binary.
  def make_wasm(*sections)
    all = sections.reduce(WASM_HEADER.dup) { |acc, s| acc + s }
    all
  end

  # Value type constants
  I32     = 0x7F
  I64     = 0x7E
  F32     = 0x7D
  F64     = 0x7C
  FUNCREF = 0x70

  # Section IDs
  SEC_CUSTOM   = 0
  SEC_TYPE     = 1
  SEC_IMPORT   = 2
  SEC_FUNCTION = 3
  SEC_TABLE    = 4
  SEC_MEMORY   = 5
  SEC_GLOBAL   = 6
  SEC_EXPORT   = 7
  SEC_START    = 8
  SEC_ELEMENT  = 9
  SEC_CODE     = 10
  SEC_DATA     = 11

  # External kind bytes
  KIND_FUNC   = 0x00
  KIND_TABLE  = 0x01
  KIND_MEMORY = 0x02
  KIND_GLOBAL = 0x03

  def parser
    @parser ||= CodingAdventures::WasmModuleParser::Parser.new
  end

  # ── Test: version ──────────────────────────────────────────────────────

  def test_version_exists
    refute_nil CodingAdventures::WasmModuleParser::VERSION
    assert_equal "0.1.0", CodingAdventures::WasmModuleParser::VERSION
  end

  # ── Test 1: Minimal module ────────────────────────────────────────────

  def test_01_minimal_module
    wasm = make_wasm  # header only, no sections
    mod  = parser.parse(wasm)

    assert_empty mod.types
    assert_empty mod.imports
    assert_empty mod.functions
    assert_empty mod.tables
    assert_empty mod.memories
    assert_empty mod.globals
    assert_empty mod.exports
    assert_nil   mod.start
    assert_empty mod.elements
    assert_empty mod.code
    assert_empty mod.data
    assert_empty mod.customs
  end

  # ── Test 2: Type section ───────────────────────────────────────────────

  def test_02_type_section_i32_i32_to_i32
    type_payload = [
      1,              # count = 1
      0x60,           # function type prefix
      2, I32, I32,    # params: i32, i32
      1, I32          # results: i32
    ]
    wasm = make_wasm(make_section(SEC_TYPE, type_payload))
    mod  = parser.parse(wasm)

    assert_equal 1, mod.types.length
    assert_equal [I32, I32], mod.types[0].params
    assert_equal [I32],      mod.types[0].results
  end

  def test_02b_type_section_multiple_types
    type_payload = [
      2,              # count = 2
      0x60, 0, 0,     # () → ()
      0x60, 1, I64, 1, F64  # (i64) → f64
    ]
    wasm = make_wasm(make_section(SEC_TYPE, type_payload))
    mod  = parser.parse(wasm)

    assert_equal 2, mod.types.length
    assert_empty mod.types[0].params
    assert_empty mod.types[0].results
    assert_equal [I64], mod.types[1].params
    assert_equal [F64], mod.types[1].results
  end

  # ── Test 3: Function section ───────────────────────────────────────────

  def test_03_function_section_type_indices
    type_payload = [1, 0x60, 0, 0]
    func_payload = [2, 0, 0]  # 2 functions, both using type 0
    wasm = make_wasm(
      make_section(SEC_TYPE, type_payload),
      make_section(SEC_FUNCTION, func_payload)
    )
    mod = parser.parse(wasm)

    assert_equal 2, mod.functions.length
    assert_equal 0, mod.functions[0]
    assert_equal 0, mod.functions[1]
  end

  # ── Test 4: Export section ─────────────────────────────────────────────

  def test_04_export_section_function_named_main
    type_payload   = [1, 0x60, 0, 0]
    func_payload   = [1, 0]
    export_payload = [1, *make_string("main"), KIND_FUNC, 0]
    wasm = make_wasm(
      make_section(SEC_TYPE,     type_payload),
      make_section(SEC_FUNCTION, func_payload),
      make_section(SEC_EXPORT,   export_payload)
    )
    mod = parser.parse(wasm)

    assert_equal 1,        mod.exports.length
    assert_equal "main",   mod.exports[0].name
    assert_equal :function, mod.exports[0].kind
    assert_equal 0,        mod.exports[0].index
  end

  # ── Test 5: Code section ───────────────────────────────────────────────

  def test_05_code_section_with_locals_and_instructions
    type_payload = [1, 0x60, 2, I32, I32, 1, I32]
    func_payload = [1, 0]
    body_bytes = [
      1,            # 1 local group
      1, I32,       # 1 × i32 local
      0x20, 0x00,   # local.get 0
      0x20, 0x01,   # local.get 1
      0x6A,         # i32.add
      0x0B          # end
    ]
    code_payload = [1, *encode_uleb128(body_bytes.length), *body_bytes]
    wasm = make_wasm(
      make_section(SEC_TYPE,     type_payload),
      make_section(SEC_FUNCTION, func_payload),
      make_section(SEC_CODE,     code_payload)
    )
    mod = parser.parse(wasm)

    assert_equal 1,    mod.code.length
    assert_equal [I32], mod.code[0].locals
    expected_code = [0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B].pack("C*").b
    assert_equal expected_code, mod.code[0].code
  end

  def test_05b_code_section_multiple_local_groups
    type_payload = [1, 0x60, 0, 0]
    func_payload = [1, 0]
    body_bytes   = [
      2,            # 2 local groups
      2, I32,       # 2 × i32
      1, F64,       # 1 × f64
      0x0B          # end
    ]
    code_payload = [1, *encode_uleb128(body_bytes.length), *body_bytes]
    wasm = make_wasm(
      make_section(SEC_TYPE,     type_payload),
      make_section(SEC_FUNCTION, func_payload),
      make_section(SEC_CODE,     code_payload)
    )
    mod = parser.parse(wasm)

    assert_equal [I32, I32, F64], mod.code[0].locals
  end

  # ── Test 6: Import section ─────────────────────────────────────────────

  def test_06_import_section_function
    type_payload   = [1, 0x60, 0, 0]
    import_payload = [1, *make_string("env"), *make_string("print"), KIND_FUNC, 0]
    wasm = make_wasm(
      make_section(SEC_TYPE,   type_payload),
      make_section(SEC_IMPORT, import_payload)
    )
    mod = parser.parse(wasm)

    assert_equal 1,         mod.imports.length
    assert_equal "env",     mod.imports[0].module_name
    assert_equal "print",   mod.imports[0].name
    assert_equal :function, mod.imports[0].kind
    assert_equal 0,         mod.imports[0].type_info
  end

  def test_06b_import_section_memory
    import_payload = [1, *make_string("env"), *make_string("mem"), KIND_MEMORY, 0x00, 1]
    wasm = make_wasm(make_section(SEC_IMPORT, import_payload))
    mod  = parser.parse(wasm)

    assert_equal :memory, mod.imports[0].kind
    assert_equal 1,       mod.imports[0].type_info.limits.min
    assert_nil            mod.imports[0].type_info.limits.max
  end

  def test_06c_import_section_table
    import_payload = [1, *make_string("env"), *make_string("tbl"), KIND_TABLE, FUNCREF, 0x00, 10]
    wasm = make_wasm(make_section(SEC_IMPORT, import_payload))
    mod  = parser.parse(wasm)

    assert_equal :table,  mod.imports[0].kind
    assert_equal FUNCREF, mod.imports[0].type_info.element_type
    assert_equal 10,      mod.imports[0].type_info.limits.min
  end

  def test_06d_import_section_global
    import_payload = [1, *make_string("env"), *make_string("sp"), KIND_GLOBAL, I32, 0x01]
    wasm = make_wasm(make_section(SEC_IMPORT, import_payload))
    mod  = parser.parse(wasm)

    assert_equal :global, mod.imports[0].kind
    assert_equal I32,     mod.imports[0].type_info.value_type
    assert_equal true,    mod.imports[0].type_info.mutable
  end

  # ── Test 7: Memory section ─────────────────────────────────────────────

  def test_07_memory_section_no_max
    mem_payload = [1, 0x00, 1]  # flags=0, min=1
    wasm = make_wasm(make_section(SEC_MEMORY, mem_payload))
    mod  = parser.parse(wasm)

    assert_equal 1, mod.memories.length
    assert_equal 1, mod.memories[0].limits.min
    assert_nil      mod.memories[0].limits.max
  end

  def test_07b_memory_section_with_max
    mem_payload = [1, 0x01, 2, 4]  # flags=1, min=2, max=4
    wasm = make_wasm(make_section(SEC_MEMORY, mem_payload))
    mod  = parser.parse(wasm)

    assert_equal 2, mod.memories[0].limits.min
    assert_equal 4, mod.memories[0].limits.max
  end

  # ── Test 8: Table section ──────────────────────────────────────────────

  def test_08_table_section_funcref_min_10
    table_payload = [1, FUNCREF, 0x00, 10]  # 1 table, funcref, flags=0, min=10
    wasm = make_wasm(make_section(SEC_TABLE, table_payload))
    mod  = parser.parse(wasm)

    assert_equal 1,       mod.tables.length
    assert_equal FUNCREF, mod.tables[0].element_type
    assert_equal 10,      mod.tables[0].limits.min
    assert_nil            mod.tables[0].limits.max
  end

  # ── Test 9: Global section ─────────────────────────────────────────────

  def test_09_global_section_immutable_i32_42
    # i32.const 42; end = [0x41, 0x2A, 0x0B]
    global_payload = [1, I32, 0x00, 0x41, 42, 0x0B]
    wasm = make_wasm(make_section(SEC_GLOBAL, global_payload))
    mod  = parser.parse(wasm)

    assert_equal 1,     mod.globals.length
    assert_equal I32,   mod.globals[0].global_type.value_type
    assert_equal false, mod.globals[0].global_type.mutable
    assert_equal [0x41, 42, 0x0B].pack("C*").b, mod.globals[0].init_expr
  end

  def test_09b_global_section_mutable_i64
    global_payload = [1, I64, 0x01, 0x42, 0x00, 0x0B]  # mutable i64 = 0
    wasm = make_wasm(make_section(SEC_GLOBAL, global_payload))
    mod  = parser.parse(wasm)

    assert_equal I64,  mod.globals[0].global_type.value_type
    assert_equal true, mod.globals[0].global_type.mutable
  end

  # ── Test 10: Data section ──────────────────────────────────────────────

  def test_10_data_section_write_hi_at_0
    data_payload = [1, 0, 0x41, 0x00, 0x0B, 2, 0x48, 0x69]
    #               ^ count  ^ mem_idx  ^ offset_expr (i32.const 0; end)  ^ 2 bytes "Hi"
    wasm = make_wasm(make_section(SEC_DATA, data_payload))
    mod  = parser.parse(wasm)

    assert_equal 1, mod.data.length
    assert_equal 0, mod.data[0].memory_index
    assert_equal [0x41, 0x00, 0x0B].pack("C*").b, mod.data[0].offset_expr
    assert_equal [0x48, 0x69].pack("C*").b,        mod.data[0].data
  end

  # ── Test 11: Element section ───────────────────────────────────────────

  def test_11_element_section_fill_table_with_functions
    elem_payload = [1, 0, 0x41, 0x00, 0x0B, 2, 0, 1]
    #               ^ count  ^ table_idx  ^ offset (i32.const 0)  ^ 2 funcs: 0, 1
    wasm = make_wasm(make_section(SEC_ELEMENT, elem_payload))
    mod  = parser.parse(wasm)

    assert_equal 1,     mod.elements.length
    assert_equal 0,     mod.elements[0].table_index
    assert_equal [0, 1], mod.elements[0].function_indices
  end

  # ── Test 12: Start section ─────────────────────────────────────────────

  def test_12_start_section_function_index_2
    start_payload = [2]  # function_index = 2
    wasm = make_wasm(make_section(SEC_START, start_payload))
    mod  = parser.parse(wasm)

    assert_equal 2, mod.start
  end

  def test_12b_start_is_nil_when_absent
    wasm = make_wasm
    mod  = parser.parse(wasm)
    assert_nil mod.start
  end

  # ── Test 13: Custom section ────────────────────────────────────────────

  def test_13_custom_section_name_and_data
    custom_payload = [*make_string("name"), 0x01, 0x02, 0x03]
    wasm = make_wasm(make_section(SEC_CUSTOM, custom_payload))
    mod  = parser.parse(wasm)

    assert_equal 1,    mod.customs.length
    assert_equal "name", mod.customs[0].name
    assert_equal [0x01, 0x02, 0x03].pack("C*").b, mod.customs[0].data
  end

  def test_13b_multiple_custom_sections
    custom1 = make_section(SEC_CUSTOM, [*make_string("debug"), 0xAA])
    custom2 = make_section(SEC_CUSTOM, [*make_string("producers"), 0xBB, 0xCC])
    wasm = make_wasm(custom1, custom2)
    mod  = parser.parse(wasm)

    assert_equal 2,          mod.customs.length
    assert_equal "debug",     mod.customs[0].name
    assert_equal "producers", mod.customs[1].name
  end

  # ── Test 14: Multi-section module ─────────────────────────────────────

  def test_14_multi_section_module
    type_payload   = [1, 0x60, 1, I32, 1, I32]
    import_payload = [1, *make_string("env"), *make_string("add"), KIND_FUNC, 0]
    func_payload   = [1, 0]
    export_payload = [1, *make_string("double"), KIND_FUNC, 1]
    body_bytes     = [0, 0x20, 0x00, 0x10, 0x00, 0x0B]  # 0 locals; local.get 0; call 0; end
    code_payload   = [1, *encode_uleb128(body_bytes.length), *body_bytes]

    wasm = make_wasm(
      make_section(SEC_TYPE,     type_payload),
      make_section(SEC_IMPORT,   import_payload),
      make_section(SEC_FUNCTION, func_payload),
      make_section(SEC_EXPORT,   export_payload),
      make_section(SEC_CODE,     code_payload)
    )
    mod = parser.parse(wasm)

    assert_equal 1, mod.types.length
    assert_equal 1, mod.imports.length
    assert_equal 1, mod.functions.length
    assert_equal 1, mod.exports.length
    assert_equal 1, mod.code.length
    assert_equal "double", mod.exports[0].name
  end

  # ── Test 15: Error: bad magic ─────────────────────────────────────────

  def test_15_bad_magic_bytes
    bad = [0x00, 0x61, 0x73, 0x6E, 0x01, 0x00, 0x00, 0x00]  # 'n' not 'm'
    err = assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(bad)
    end
    assert_equal 3, err.offset
  end

  def test_15b_wrong_first_magic_byte
    bad = [0xFF, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]
    err = assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(bad)
    end
    assert_equal 0, err.offset
  end

  # ── Test 16: Error: wrong version ─────────────────────────────────────

  def test_16_wrong_version
    bad = [0x00, 0x61, 0x73, 0x6D, 0x02, 0x00, 0x00, 0x00]  # version=2
    err = assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(bad)
    end
    assert_equal 4, err.offset
  end

  # ── Test 17: Error: truncated header ──────────────────────────────────

  def test_17_truncated_header
    bad = [0x00, 0x61, 0x73, 0x6D]  # only 4 bytes
    err = assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(bad)
    end
    assert_equal 0, err.offset
  end

  def test_17b_empty_input
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse([])
    end
  end

  # ── Test 18: Error: truncated section ─────────────────────────────────

  def test_18_truncated_section_payload
    # Type section claims 100 bytes but only 1 is present
    truncated = [*WASM_HEADER, SEC_TYPE, 100, 1]
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(truncated)
    end
  end

  # ── Test 19: Round-trip test ───────────────────────────────────────────

  def test_19_round_trip_all_sections
    type_payload   = [1, 0x60, 1, I32, 1, I32]
    import_payload = [1, *make_string("env"), *make_string("abort"), KIND_FUNC, 0]
    func_payload   = [1, 0]
    table_payload  = [1, FUNCREF, 0x00, 1]
    mem_payload    = [1, 0x00, 1]
    global_payload = [1, I32, 0x00, 0x41, 0x00, 0x0B]
    export_payload = [1, *make_string("identity"), KIND_FUNC, 1]
    start_payload  = [1]  # start = function 1
    elem_payload   = [1, 0, 0x41, 0x00, 0x0B, 1, 1]
    body_bytes     = [0, 0x20, 0x00, 0x0B]  # no locals; local.get 0; end
    code_payload   = [1, *encode_uleb128(body_bytes.length), *body_bytes]
    data_payload   = [1, 0, 0x41, 0x00, 0x0B, 1, 0x42]
    custom_payload = [*make_string("debug"), 0xDE, 0xAD]

    wasm = make_wasm(
      make_section(SEC_CUSTOM,   custom_payload),  # custom before type section (allowed)
      make_section(SEC_TYPE,     type_payload),
      make_section(SEC_IMPORT,   import_payload),
      make_section(SEC_FUNCTION, func_payload),
      make_section(SEC_TABLE,    table_payload),
      make_section(SEC_MEMORY,   mem_payload),
      make_section(SEC_GLOBAL,   global_payload),
      make_section(SEC_EXPORT,   export_payload),
      make_section(SEC_START,    start_payload),
      make_section(SEC_ELEMENT,  elem_payload),
      make_section(SEC_CODE,     code_payload),
      make_section(SEC_DATA,     data_payload)
    )
    mod = parser.parse(wasm)

    # Types
    assert_equal 1,     mod.types.length
    assert_equal [I32], mod.types[0].params
    assert_equal [I32], mod.types[0].results

    # Imports
    assert_equal 1,         mod.imports.length
    assert_equal "env",     mod.imports[0].module_name
    assert_equal "abort",   mod.imports[0].name
    assert_equal :function, mod.imports[0].kind

    # Functions
    assert_equal 1, mod.functions.length
    assert_equal 0, mod.functions[0]

    # Tables
    assert_equal 1,       mod.tables.length
    assert_equal FUNCREF, mod.tables[0].element_type
    assert_equal 1,       mod.tables[0].limits.min
    assert_nil            mod.tables[0].limits.max

    # Memories
    assert_equal 1, mod.memories.length
    assert_equal 1, mod.memories[0].limits.min
    assert_nil      mod.memories[0].limits.max

    # Globals
    assert_equal 1,     mod.globals.length
    assert_equal I32,   mod.globals[0].global_type.value_type
    assert_equal false, mod.globals[0].global_type.mutable

    # Exports
    assert_equal 1,          mod.exports.length
    assert_equal "identity", mod.exports[0].name
    assert_equal 1,          mod.exports[0].index

    # Start
    assert_equal 1, mod.start

    # Elements
    assert_equal 1,   mod.elements.length
    assert_equal [1], mod.elements[0].function_indices

    # Code
    assert_equal 1, mod.code.length
    assert_empty    mod.code[0].locals
    assert_equal [0x20, 0x00, 0x0B].pack("C*").b, mod.code[0].code

    # Data
    assert_equal 1, mod.data.length
    assert_equal 0, mod.data[0].memory_index
    assert_equal [0x42].pack("C*").b, mod.data[0].data

    # Customs
    assert_equal 1,       mod.customs.length
    assert_equal "debug", mod.customs[0].name
    assert_equal [0xDE, 0xAD].pack("C*").b, mod.customs[0].data
  end

  # ── Additional error coverage ──────────────────────────────────────────

  def test_wasm_parse_error_has_offset
    err = CodingAdventures::WasmModuleParser::WasmParseError.new("test", 42)
    assert_equal 42,     err.offset
    assert_equal "test", err.message
    assert_kind_of StandardError, err
  end

  def test_parse_accepts_binary_string
    wasm_str = WASM_HEADER.pack("C*").b
    mod = parser.parse(wasm_str)
    assert_empty mod.types
  end

  def test_empty_sections_graceful
    # Type section with 0 types
    wasm = make_wasm(make_section(SEC_TYPE, [0]))
    mod  = parser.parse(wasm)
    assert_empty mod.types
  end

  def test_large_leb128_count
    # 200 trivial () → () types; 200 in ULEB128 = [0xC8, 0x01]
    types = 200.times.flat_map { [0x60, 0, 0] }
    type_payload = [0xC8, 0x01, *types]
    wasm = make_wasm(make_section(SEC_TYPE, type_payload))
    mod  = parser.parse(wasm)
    assert_equal 200, mod.types.length
  end

  def test_all_four_value_types_in_signature
    type_payload = [1, 0x60, 4, I32, I64, F32, F64, 4, F64, F32, I64, I32]
    wasm = make_wasm(make_section(SEC_TYPE, type_payload))
    mod  = parser.parse(wasm)
    assert_equal [I32, I64, F32, F64], mod.types[0].params
    assert_equal [F64, F32, I64, I32], mod.types[0].results
  end

  def test_all_four_export_kinds
    table_payload  = [1, FUNCREF, 0x00, 1]
    mem_payload    = [1, 0x00, 1]
    global_payload = [1, I32, 0x00, 0x41, 0x00, 0x0B]
    type_payload   = [1, 0x60, 0, 0]
    func_payload   = [1, 0]
    body_bytes     = [0, 0x0B]
    code_payload   = [1, *encode_uleb128(body_bytes.length), *body_bytes]
    export_payload = [
      4,
      *make_string("fn"),  KIND_FUNC,   0,
      *make_string("tbl"), KIND_TABLE,  0,
      *make_string("mem"), KIND_MEMORY, 0,
      *make_string("glb"), KIND_GLOBAL, 0
    ]
    wasm = make_wasm(
      make_section(SEC_TYPE,     type_payload),
      make_section(SEC_FUNCTION, func_payload),
      make_section(SEC_TABLE,    table_payload),
      make_section(SEC_MEMORY,   mem_payload),
      make_section(SEC_GLOBAL,   global_payload),
      make_section(SEC_EXPORT,   export_payload),
      make_section(SEC_CODE,     code_payload)
    )
    mod = parser.parse(wasm)
    assert_equal 4,        mod.exports.length
    assert_equal :function, mod.exports[0].kind
    assert_equal :table,    mod.exports[1].kind
    assert_equal :memory,   mod.exports[2].kind
    assert_equal :global,   mod.exports[3].kind
  end

  def test_custom_section_with_empty_data
    custom_payload = make_string("empty")
    wasm = make_wasm(make_section(SEC_CUSTOM, custom_payload))
    mod  = parser.parse(wasm)
    assert_equal "empty", mod.customs[0].name
    assert_equal "".b,    mod.customs[0].data
  end

  def test_throws_on_out_of_order_sections
    type_payload   = [1, 0x60, 0, 0]
    func_payload   = [1, 0]
    export_payload = [1, *make_string("fn"), KIND_FUNC, 0]
    # Put export (7) then function (3) — out of order
    bad = [
      *WASM_HEADER,
      *make_section(SEC_TYPE,     type_payload),
      *make_section(SEC_EXPORT,   export_payload),  # id=7
      *make_section(SEC_FUNCTION, func_payload)      # id=3 after id=7 → error
    ]
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(bad)
    end
  end

  def test_silently_skips_unknown_section_ids
    unknown_section = make_section(99, [0xAA, 0xBB])
    wasm = make_wasm(unknown_section)
    mod  = parser.parse(wasm)
    assert_empty mod.types
    assert_empty mod.customs
  end

  def test_throws_on_invalid_type_prefix
    type_payload = [1, 0x61, 0, 0]  # 0x61 instead of 0x60
    wasm = make_wasm(make_section(SEC_TYPE, type_payload))
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(wasm)
    end
  end

  def test_throws_on_invalid_value_type_in_params
    type_payload = [1, 0x60, 1, 0x00, 0]  # 0x00 is not a valid value type
    wasm = make_wasm(make_section(SEC_TYPE, type_payload))
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(wasm)
    end
  end

  def test_throws_on_invalid_export_kind
    type_payload   = [1, 0x60, 0, 0]
    func_payload   = [1, 0]
    export_payload = [1, *make_string("fn"), 0x99, 0]  # 0x99 invalid kind
    body_bytes     = [0, 0x0B]
    code_payload   = [1, *encode_uleb128(body_bytes.length), *body_bytes]
    wasm = make_wasm(
      make_section(SEC_TYPE,     type_payload),
      make_section(SEC_FUNCTION, func_payload),
      make_section(SEC_EXPORT,   export_payload),
      make_section(SEC_CODE,     code_payload)
    )
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(wasm)
    end
  end

  def test_throws_on_invalid_import_kind
    type_payload   = [1, 0x60, 0, 0]
    import_payload = [1, *make_string("env"), *make_string("x"), 0x99, 0]  # 0x99 invalid
    wasm = make_wasm(
      make_section(SEC_TYPE,   type_payload),
      make_section(SEC_IMPORT, import_payload)
    )
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(wasm)
    end
  end

  def test_throws_on_init_expr_without_end_opcode
    # Global with init expr that never terminates with 0x0B
    global_payload = [1, I32, 0x00, 0x41, 42]  # i32.const 42 but no 0x0B
    wasm = make_wasm(make_section(SEC_GLOBAL, global_payload))
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(wasm)
    end
  end

  def test_throws_on_invalid_table_element_type
    table_payload = [1, 0x6F, 0x00, 1]  # 0x6F not valid in WASM 1.0 table
    wasm = make_wasm(make_section(SEC_TABLE, table_payload))
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(wasm)
    end
  end

  def test_throws_on_code_body_exceeding_data_bounds
    # Claim body size=50 but only provide 2 bytes
    code_payload = [1, 50, 0, 0x0B]  # 1 body, size=50, actual=2
    wasm = make_wasm(make_section(SEC_CODE, code_payload))
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(wasm)
    end
  end

  def test_throws_on_truncated_start_section
    # Start section with 0 bytes (expects u32leb)
    wasm = make_wasm(make_section(SEC_START, []))
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(wasm)
    end
  end

  def test_throws_on_invalid_global_value_type
    # Global with invalid value type byte 0x00
    global_payload = [1, 0x00, 0x00, 0x41, 0x00, 0x0B]  # 0x00 not a valid value type
    wasm = make_wasm(make_section(SEC_GLOBAL, global_payload))
    assert_raises(CodingAdventures::WasmModuleParser::WasmParseError) do
      parser.parse(wasm)
    end
  end
end
