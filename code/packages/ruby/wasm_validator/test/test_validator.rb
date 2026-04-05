# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_validator"

# ==========================================================================
# Tests for the WASM Validator
# ==========================================================================
#
# The validator checks a parsed WasmModule for structural correctness:
#
#   - At most one memory and one table (WASM 1.0 restriction)
#   - Memory limits within bounds (max 65536 pages)
#   - Export names are unique
#   - Valid modules produce ValidatedModule
#
# Tests cover:
#   1. Valid minimal module
#   2. Valid module with one memory
#   3. Valid module with one table
#   4. Multiple memories rejected
#   5. Multiple tables rejected
#   6. Memory limit exceeded
#   7. Memory min > max rejected
#   8. Duplicate export names rejected
#   9. Imported memories and tables count toward limits
#   10. ValidatedModule has correct func_types
# ==========================================================================

class TestValidator < Minitest::Test
  WV = CodingAdventures::WasmValidator
  WT = CodingAdventures::WasmTypes
  ValidationError = CodingAdventures::WasmValidator::ValidationError

  def make_module
    WT::WasmModule.new
  end

  def make_memory(min_pages, max_pages = nil)
    WT::MemoryType.new(WT::Limits.new(min_pages, max_pages))
  end

  def make_table(min_size, max_size = nil)
    WT::TableType.new(0x70, WT::Limits.new(min_size, max_size))
  end

  def make_export(name, kind = WT::EXTERNAL_KIND[:function], index = 0)
    WT::Export.new(name, kind, index)
  end

  def make_func_type(params = [], results = [])
    WT::FuncType.new(params, results)
  end

  def make_import(module_name, name, kind, type_info = nil)
    WT::Import.new(module_name, name, kind, type_info)
  end

  # ── Valid Modules ──────────────────────────────────────────────────

  def test_valid_empty_module
    mod = make_module
    result = WV.validate(mod)
    assert_kind_of WV::ValidatedModule, result
    assert_equal mod, result.wasm_module
  end

  def test_valid_module_with_one_memory
    mod = make_module
    mod.memories << make_memory(1, 10)
    result = WV.validate(mod)
    refute_nil result
  end

  def test_valid_module_with_one_table
    mod = make_module
    mod.tables << make_table(10, 100)
    result = WV.validate(mod)
    refute_nil result
  end

  def test_valid_module_with_functions
    mod = make_module
    ft = make_func_type([WT::VALUE_TYPE[:i32]], [WT::VALUE_TYPE[:i32]])
    mod.types << ft
    mod.functions << 0 # function 0 uses type 0
    result = WV.validate(mod)
    assert_equal 1, result.func_types.length
    assert_equal ft, result.func_types[0]
  end

  def test_valid_module_with_unique_exports
    mod = make_module
    mod.types << make_func_type
    mod.functions << 0
    mod.exports << make_export("foo")
    mod.exports << make_export("bar")
    result = WV.validate(mod)
    refute_nil result
  end

  def test_valid_module_memory_at_max_pages
    mod = make_module
    mod.memories << make_memory(65536, 65536)
    result = WV.validate(mod)
    refute_nil result
  end

  def test_valid_module_memory_min_equals_max
    mod = make_module
    mod.memories << make_memory(5, 5)
    result = WV.validate(mod)
    refute_nil result
  end

  # ── Invalid: Multiple Memories ─────────────────────────────────────

  def test_two_memories_rejected
    mod = make_module
    mod.memories << make_memory(1)
    mod.memories << make_memory(1)
    err = assert_raises(ValidationError) { WV.validate(mod) }
    assert_equal :multiple_memories, err.kind
  end

  def test_imported_plus_defined_memory_rejected
    mod = make_module
    mod.imports << make_import("env", "mem", WT::EXTERNAL_KIND[:memory])
    mod.memories << make_memory(1)
    err = assert_raises(ValidationError) { WV.validate(mod) }
    assert_equal :multiple_memories, err.kind
  end

  def test_two_imported_memories_rejected
    mod = make_module
    mod.imports << make_import("env", "mem1", WT::EXTERNAL_KIND[:memory])
    mod.imports << make_import("env", "mem2", WT::EXTERNAL_KIND[:memory])
    err = assert_raises(ValidationError) { WV.validate(mod) }
    assert_equal :multiple_memories, err.kind
  end

  # ── Invalid: Multiple Tables ───────────────────────────────────────

  def test_two_tables_rejected
    mod = make_module
    mod.tables << make_table(10)
    mod.tables << make_table(10)
    err = assert_raises(ValidationError) { WV.validate(mod) }
    assert_equal :multiple_tables, err.kind
  end

  def test_imported_plus_defined_table_rejected
    mod = make_module
    mod.imports << make_import("env", "tbl", WT::EXTERNAL_KIND[:table])
    mod.tables << make_table(10)
    err = assert_raises(ValidationError) { WV.validate(mod) }
    assert_equal :multiple_tables, err.kind
  end

  # ── Invalid: Memory Limits ─────────────────────────────────────────

  def test_memory_min_exceeds_max_pages
    mod = make_module
    mod.memories << make_memory(65537)
    err = assert_raises(ValidationError) { WV.validate(mod) }
    assert_equal :memory_limit_exceeded, err.kind
  end

  def test_memory_max_exceeds_max_pages
    mod = make_module
    mod.memories << make_memory(1, 65537)
    err = assert_raises(ValidationError) { WV.validate(mod) }
    assert_equal :memory_limit_exceeded, err.kind
  end

  def test_memory_min_greater_than_max
    mod = make_module
    mod.memories << make_memory(10, 5)
    err = assert_raises(ValidationError) { WV.validate(mod) }
    assert_equal :memory_limit_order, err.kind
  end

  # ── Invalid: Duplicate Exports ─────────────────────────────────────

  def test_duplicate_export_names_rejected
    mod = make_module
    mod.exports << make_export("foo")
    mod.exports << make_export("foo")
    err = assert_raises(ValidationError) { WV.validate(mod) }
    assert_equal :duplicate_export_name, err.kind
  end

  def test_same_name_different_kinds_still_duplicate
    mod = make_module
    mod.exports << make_export("thing", WT::EXTERNAL_KIND[:function], 0)
    mod.exports << make_export("thing", WT::EXTERNAL_KIND[:memory], 0)
    err = assert_raises(ValidationError) { WV.validate(mod) }
    assert_equal :duplicate_export_name, err.kind
  end

  # ── ValidatedModule structure ──────────────────────────────────────

  def test_validated_module_includes_imported_func_types
    mod = make_module
    ft_imported = make_func_type([WT::VALUE_TYPE[:i32]], [])
    ft_local = make_func_type([], [WT::VALUE_TYPE[:i32]])
    mod.types << ft_imported
    mod.types << ft_local
    mod.imports << make_import("env", "print", WT::EXTERNAL_KIND[:function], 0)
    mod.functions << 1

    result = WV.validate(mod)
    assert_equal 2, result.func_types.length
    assert_equal ft_imported, result.func_types[0]
    assert_equal ft_local, result.func_types[1]
  end

  def test_validated_module_func_types_frozen
    mod = make_module
    result = WV.validate(mod)
    assert_predicate result.func_types, :frozen?
  end

  # ── ValidationError ────────────────────────────────────────────────

  def test_validation_error_is_standard_error
    assert ValidationError < StandardError
  end

  def test_validation_error_has_kind
    err = ValidationError.new(:some_kind, "some message")
    assert_equal :some_kind, err.kind
    assert_equal "some message", err.message
  end

  # ── Version ────────────────────────────────────────────────────────

  def test_version_exists
    refute_nil CodingAdventures::WasmValidator::VERSION
  end
end
