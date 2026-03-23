# frozen_string_literal: true

# test_wasm_types.rb — Tests for the WASM 1.0 type system (Ruby)
#
# Covers:
#   - VALUE_TYPE hash values match spec bytes
#   - BLOCK_TYPE_EMPTY = 0x40
#   - EXTERNAL_KIND hash values
#   - FuncType construction
#   - FuncType with empty params/results
#   - FuncType with multiple params
#   - Limits with only min (max = nil)
#   - Limits with min and max
#   - MemoryType construction
#   - TableType construction
#   - GlobalType mutable and immutable
#   - Import for each ExternalKind
#   - Export construction
#   - Global with init_expr bytes
#   - Element with function_indices
#   - DataSegment construction
#   - FunctionBody with locals and code
#   - CustomSection construction
#   - WasmModule starts empty
#   - WasmModule can be populated

require "minitest/autorun"
require "coding_adventures_wasm_types"

class TestWasmTypes < Minitest::Test
  include CodingAdventures::WasmTypes

  # ─────────────────────────────────────────────────────────────────────────
  # VERSION
  # ─────────────────────────────────────────────────────────────────────────

  def test_version_exists
    refute_nil CodingAdventures::WasmTypes::VERSION
  end

  # ─────────────────────────────────────────────────────────────────────────
  # VALUE_TYPE
  # ─────────────────────────────────────────────────────────────────────────

  def test_value_type_i32
    assert_equal 0x7F, VALUE_TYPE[:i32]
  end

  def test_value_type_i64
    assert_equal 0x7E, VALUE_TYPE[:i64]
  end

  def test_value_type_f32
    assert_equal 0x7D, VALUE_TYPE[:f32]
  end

  def test_value_type_f64
    assert_equal 0x7C, VALUE_TYPE[:f64]
  end

  def test_value_type_all_distinct
    codes = VALUE_TYPE.values
    assert_equal codes.length, codes.uniq.length
  end

  def test_value_type_descending_order
    # WASM spec: i32 > i64 > f32 > f64
    assert_operator VALUE_TYPE[:i32], :>, VALUE_TYPE[:i64]
    assert_operator VALUE_TYPE[:i64], :>, VALUE_TYPE[:f32]
    assert_operator VALUE_TYPE[:f32], :>, VALUE_TYPE[:f64]
  end

  def test_value_type_is_frozen
    assert VALUE_TYPE.frozen?
  end

  # ─────────────────────────────────────────────────────────────────────────
  # BLOCK_TYPE_EMPTY
  # ─────────────────────────────────────────────────────────────────────────

  def test_block_type_empty
    assert_equal 0x40, BLOCK_TYPE_EMPTY
  end

  # ─────────────────────────────────────────────────────────────────────────
  # EXTERNAL_KIND
  # ─────────────────────────────────────────────────────────────────────────

  def test_external_kind_function
    assert_equal 0x00, EXTERNAL_KIND[:function]
  end

  def test_external_kind_table
    assert_equal 0x01, EXTERNAL_KIND[:table]
  end

  def test_external_kind_memory
    assert_equal 0x02, EXTERNAL_KIND[:memory]
  end

  def test_external_kind_global
    assert_equal 0x03, EXTERNAL_KIND[:global]
  end

  def test_external_kind_all_distinct
    kinds = EXTERNAL_KIND.values
    assert_equal kinds.length, kinds.uniq.length
  end

  def test_external_kind_is_frozen
    assert EXTERNAL_KIND.frozen?
  end

  # ─────────────────────────────────────────────────────────────────────────
  # FUNCREF
  # ─────────────────────────────────────────────────────────────────────────

  def test_funcref_value
    assert_equal 0x70, FUNCREF
  end

  # ─────────────────────────────────────────────────────────────────────────
  # FuncType
  # ─────────────────────────────────────────────────────────────────────────

  def test_func_type_construction
    ft = FuncType.new([VALUE_TYPE[:i32], VALUE_TYPE[:i64]], [VALUE_TYPE[:f64]])
    assert_equal [VALUE_TYPE[:i32], VALUE_TYPE[:i64]], ft.params
    assert_equal [VALUE_TYPE[:f64]], ft.results
  end

  def test_func_type_empty_params_and_results
    ft = FuncType.new([], [])
    assert_equal [], ft.params
    assert_equal [], ft.results
    assert_equal 0, ft.params.length
    assert_equal 0, ft.results.length
  end

  def test_func_type_multiple_params
    ft = FuncType.new(
      [VALUE_TYPE[:i32], VALUE_TYPE[:i32], VALUE_TYPE[:i32]],
      [VALUE_TYPE[:i32]]
    )
    assert_equal 3, ft.params.length
    assert_equal 1, ft.results.length
    assert_equal VALUE_TYPE[:i32], ft.params[0]
    assert_equal VALUE_TYPE[:i32], ft.params[1]
    assert_equal VALUE_TYPE[:i32], ft.params[2]
  end

  def test_func_type_only_results
    ft = FuncType.new([], [VALUE_TYPE[:i32]])
    assert_equal [], ft.params
    assert_equal [VALUE_TYPE[:i32]], ft.results
  end

  def test_func_type_equality
    ft1 = FuncType.new([VALUE_TYPE[:i32]], [VALUE_TYPE[:i64]])
    ft2 = FuncType.new([VALUE_TYPE[:i32]], [VALUE_TYPE[:i64]])
    assert_equal ft1, ft2
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Limits
  # ─────────────────────────────────────────────────────────────────────────

  def test_limits_only_min
    lim = Limits.new(1, nil)
    assert_equal 1, lim.min
    assert_nil lim.max
  end

  def test_limits_with_max
    lim = Limits.new(2, 16)
    assert_equal 2, lim.min
    assert_equal 16, lim.max
  end

  def test_limits_min_zero
    lim = Limits.new(0, nil)
    assert_equal 0, lim.min
    assert_nil lim.max
  end

  def test_limits_fixed_size
    lim = Limits.new(4, 4)
    assert_equal lim.min, lim.max
  end

  # ─────────────────────────────────────────────────────────────────────────
  # MemoryType
  # ─────────────────────────────────────────────────────────────────────────

  def test_memory_type_unbounded
    mt = MemoryType.new(Limits.new(1, nil))
    assert_equal 1, mt.limits.min
    assert_nil mt.limits.max
  end

  def test_memory_type_bounded
    mt = MemoryType.new(Limits.new(1, 8))
    assert_equal 1, mt.limits.min
    assert_equal 8, mt.limits.max
  end

  # ─────────────────────────────────────────────────────────────────────────
  # TableType
  # ─────────────────────────────────────────────────────────────────────────

  def test_table_type_unbounded
    tt = TableType.new(FUNCREF, Limits.new(10, nil))
    assert_equal 0x70, tt.element_type
    assert_equal 10, tt.limits.min
    assert_nil tt.limits.max
  end

  def test_table_type_bounded
    tt = TableType.new(FUNCREF, Limits.new(0, 100))
    assert_equal FUNCREF, tt.element_type
    assert_equal 100, tt.limits.max
  end

  # ─────────────────────────────────────────────────────────────────────────
  # GlobalType
  # ─────────────────────────────────────────────────────────────────────────

  def test_global_type_immutable
    gt = GlobalType.new(VALUE_TYPE[:i32], false)
    assert_equal VALUE_TYPE[:i32], gt.value_type
    assert_equal false, gt.mutable
  end

  def test_global_type_mutable
    gt = GlobalType.new(VALUE_TYPE[:f64], true)
    assert_equal VALUE_TYPE[:f64], gt.value_type
    assert_equal true, gt.mutable
  end

  def test_global_type_mutable_distinguishable
    immut = GlobalType.new(VALUE_TYPE[:i32], false)
    mut   = GlobalType.new(VALUE_TYPE[:i32], true)
    refute_equal immut.mutable, mut.mutable
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Import
  # ─────────────────────────────────────────────────────────────────────────

  def test_import_function
    imp = Import.new("env", "add", :function, 2)
    assert_equal "env", imp.module_name
    assert_equal "add", imp.name
    assert_equal :function, imp.kind
    assert_equal 2, imp.type_info
  end

  def test_import_table
    tt  = TableType.new(FUNCREF, Limits.new(1, nil))
    imp = Import.new("env", "table", :table, tt)
    assert_equal :table, imp.kind
    assert_equal tt, imp.type_info
  end

  def test_import_memory
    mt  = MemoryType.new(Limits.new(1, nil))
    imp = Import.new("env", "memory", :memory, mt)
    assert_equal :memory, imp.kind
    assert_equal mt, imp.type_info
  end

  def test_import_global
    gt  = GlobalType.new(VALUE_TYPE[:i32], false)
    imp = Import.new("env", "stackPointer", :global, gt)
    assert_equal :global, imp.kind
    assert_equal gt, imp.type_info
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Export
  # ─────────────────────────────────────────────────────────────────────────

  def test_export_function
    exp = Export.new("main", :function, 0)
    assert_equal "main", exp.name
    assert_equal :function, exp.kind
    assert_equal 0, exp.index
  end

  def test_export_memory
    exp = Export.new("memory", :memory, 0)
    assert_equal :memory, exp.kind
    assert_equal 0, exp.index
  end

  def test_export_global
    exp = Export.new("stackPointer", :global, 1)
    assert_equal :global, exp.kind
    assert_equal 1, exp.index
  end

  def test_export_table
    exp = Export.new("__indirect_function_table", :table, 0)
    assert_equal :table, exp.kind
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Global
  # ─────────────────────────────────────────────────────────────────────────

  def test_global_construction
    # i32.const 42; end  →  \x41\x2A\x0B
    gt   = GlobalType.new(VALUE_TYPE[:i32], false)
    glob = Global.new(gt, "\x41\x2A\x0B".b)
    assert_equal VALUE_TYPE[:i32], glob.global_type.value_type
    assert_equal false, glob.global_type.mutable
    assert_equal "\x41\x2A\x0B".b, glob.init_expr
  end

  def test_global_mutable_f64
    # f64.const 0.0; end  →  \x44 + 8 zero bytes + \x0B
    init = "\x44\x00\x00\x00\x00\x00\x00\x00\x00\x0B".b
    gt   = GlobalType.new(VALUE_TYPE[:f64], true)
    glob = Global.new(gt, init)
    assert_equal true, glob.global_type.mutable
    assert_equal 10, glob.init_expr.bytesize
    assert_equal 0x44.chr, glob.init_expr[0]   # f64.const opcode
    assert_equal 0x0B.chr, glob.init_expr[-1]   # end opcode
  end

  # ─────────────────────────────────────────────────────────────────────────
  # Element
  # ─────────────────────────────────────────────────────────────────────────

  def test_element_construction
    # i32.const 0; end  →  \x41\x00\x0B
    elem = Element.new(0, "\x41\x00\x0B".b, [0, 1, 2])
    assert_equal 0, elem.table_index
    assert_equal "\x41\x00\x0B".b, elem.offset_expr
    assert_equal [0, 1, 2], elem.function_indices
    assert_equal 3, elem.function_indices.length
  end

  def test_element_empty_function_indices
    elem = Element.new(0, "\x41\x00\x0B".b, [])
    assert_equal 0, elem.function_indices.length
  end

  def test_element_non_zero_offset
    # i32.const 5; end  →  \x41\x05\x0B
    elem = Element.new(0, "\x41\x05\x0B".b, [10, 11])
    assert_equal 0x05, elem.offset_expr.bytes[1]
    assert_equal [10, 11], elem.function_indices
  end

  # ─────────────────────────────────────────────────────────────────────────
  # DataSegment
  # ─────────────────────────────────────────────────────────────────────────

  def test_data_segment_construction
    seg = DataSegment.new(0, "\x41\x00\x0B".b, "Hi".b)
    assert_equal 0, seg.memory_index
    assert_equal "\x41\x00\x0B".b, seg.offset_expr
    assert_equal "Hi".b, seg.data
    assert_equal 2, seg.data.bytesize
  end

  def test_data_segment_empty_data
    seg = DataSegment.new(0, "\x41\x00\x0B".b, "".b)
    assert_equal 0, seg.data.bytesize
  end

  def test_data_segment_non_zero_offset
    # i32.const 256; end (LEB128: \x80\x02)
    seg = DataSegment.new(0, "\x41\x80\x02\x0B".b, "\xDE\xAD\xBE\xEF".b)
    assert_equal 4, seg.offset_expr.bytesize
    assert_equal "\xDE\xAD\xBE\xEF".b, seg.data
  end

  # ─────────────────────────────────────────────────────────────────────────
  # FunctionBody
  # ─────────────────────────────────────────────────────────────────────────

  def test_function_body_with_locals_and_code
    # local.get 0; local.get 1; i32.add; end
    body = FunctionBody.new(
      [VALUE_TYPE[:i32], VALUE_TYPE[:i32]],
      "\x20\x00\x20\x01\x6A\x0B".b
    )
    assert_equal [VALUE_TYPE[:i32], VALUE_TYPE[:i32]], body.locals
    assert_equal "\x20\x00\x20\x01\x6A\x0B".b, body.code
    assert_equal 2, body.locals.length
    assert_equal 6, body.code.bytesize
  end

  def test_function_body_no_locals
    body = FunctionBody.new([], "\x0B".b)
    assert_equal 0, body.locals.length
    assert_equal 0x0B.chr, body.code[0]
  end

  def test_function_body_mixed_local_types
    body = FunctionBody.new(
      [VALUE_TYPE[:i32], VALUE_TYPE[:f64], VALUE_TYPE[:i64]],
      "\x0B".b
    )
    assert_equal VALUE_TYPE[:i32], body.locals[0]
    assert_equal VALUE_TYPE[:f64], body.locals[1]
    assert_equal VALUE_TYPE[:i64], body.locals[2]
  end

  # ─────────────────────────────────────────────────────────────────────────
  # CustomSection
  # ─────────────────────────────────────────────────────────────────────────

  def test_custom_section_construction
    cs = CustomSection.new("name", "\x01\x02\x03".b)
    assert_equal "name", cs.name
    assert_equal "\x01\x02\x03".b, cs.data
  end

  def test_custom_section_empty_data
    cs = CustomSection.new("producers", "".b)
    assert_equal "producers", cs.name
    assert_equal 0, cs.data.bytesize
  end

  def test_custom_section_any_name
    cs = CustomSection.new("my.custom.tool", "\xFF".b)
    assert_equal "my.custom.tool", cs.name
  end

  # ─────────────────────────────────────────────────────────────────────────
  # WasmModule
  # ─────────────────────────────────────────────────────────────────────────

  def test_wasm_module_starts_empty
    mod = WasmModule.new
    assert_equal [], mod.types
    assert_equal [], mod.imports
    assert_equal [], mod.functions
    assert_equal [], mod.tables
    assert_equal [], mod.memories
    assert_equal [], mod.globals
    assert_equal [], mod.exports
    assert_nil mod.start
    assert_equal [], mod.elements
    assert_equal [], mod.code
    assert_equal [], mod.data
    assert_equal [], mod.customs
  end

  def test_wasm_module_can_add_types_and_functions
    mod = WasmModule.new
    ft  = FuncType.new([VALUE_TYPE[:i32]], [VALUE_TYPE[:i32]])
    mod.types << ft
    mod.functions << 0

    assert_equal 1, mod.types.length
    assert_equal ft, mod.types[0]
    assert_equal 0, mod.functions[0]
  end

  def test_wasm_module_can_set_start
    mod = WasmModule.new
    assert_nil mod.start
    mod.start = 3
    assert_equal 3, mod.start
  end

  def test_wasm_module_can_add_memories
    mod = WasmModule.new
    mt  = MemoryType.new(Limits.new(1, nil))
    mod.memories << mt
    assert_equal 1, mod.memories.length
    assert_equal 1, mod.memories[0].limits.min
  end

  def test_wasm_module_can_add_imports_and_exports
    mod = WasmModule.new
    imp = Import.new("env", "print", :function, 0)
    exp = Export.new("main", :function, 1)
    mod.imports << imp
    mod.exports << exp
    assert_equal 1, mod.imports.length
    assert_equal 1, mod.exports.length
    assert_equal "env", mod.imports[0].module_name
    assert_equal "main", mod.exports[0].name
  end

  def test_wasm_module_can_add_globals_elements_data_customs
    mod = WasmModule.new

    glob = Global.new(GlobalType.new(VALUE_TYPE[:i32], true), "\x41\x00\x0B".b)
    elem = Element.new(0, "\x41\x00\x0B".b, [0])
    seg  = DataSegment.new(0, "\x41\x00\x0B".b, "\x2A".b)
    cs   = CustomSection.new("name", "".b)

    mod.globals  << glob
    mod.elements << elem
    mod.data     << seg
    mod.customs  << cs

    assert_equal 1, mod.globals.length
    assert_equal 1, mod.elements.length
    assert_equal 1, mod.data.length
    assert_equal 1, mod.customs.length
  end

  def test_wasm_module_instances_are_independent
    mod1 = WasmModule.new
    mod2 = WasmModule.new
    mod1.types << FuncType.new([VALUE_TYPE[:i32]], [])
    assert_equal 1, mod1.types.length
    assert_equal 0, mod2.types.length
  end
end
