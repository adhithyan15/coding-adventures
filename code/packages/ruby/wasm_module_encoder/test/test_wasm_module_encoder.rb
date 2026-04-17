# frozen_string_literal: true

require_relative "test_helper"

class TestWasmModuleEncoder < Minitest::Test
  def test_encodes_minimal_exported_function_module
    wasm_module = WT::WasmModule.new
    wasm_module.types << WT::FuncType.new([], [WT::VALUE_TYPE[:i32]])
    wasm_module.functions << 0
    wasm_module.exports << WT::Export.new("_start", WT::EXTERNAL_KIND[:function], 0)
    wasm_module.code << WT::FunctionBody.new([], [0x41, 0x00, 0x0B].pack("C*"))

    encoded = WME.encode_module(wasm_module)

    assert_equal WME::WASM_MAGIC, encoded[0, 4]
    assert_equal WME::WASM_VERSION, encoded[4, 4]
    assert_operator encoded.bytesize, :>, 8
  end

  def test_encodes_imports_memory_and_data_segments
    wasm_module = WT::WasmModule.new
    wasm_module.types << WT::FuncType.new([WT::VALUE_TYPE[:i32]], [WT::VALUE_TYPE[:i32]])
    wasm_module.imports << WT::Import.new(
      "wasi_snapshot_preview1",
      "fd_write",
      WT::EXTERNAL_KIND[:function],
      0
    )
    wasm_module.memories << WT::MemoryType.new(WT::Limits.new(1, nil))
    wasm_module.data << WT::DataSegment.new(0, [0x41, 0x00, 0x0B].pack("C*"), "A".b)

    encoded = WME.encode_module(wasm_module)

    assert_includes encoded, "wasi_snapshot_preview1"
    assert_includes encoded, "fd_write"
    assert_includes encoded, "A"
  end

  def test_groups_repeated_locals_in_function_body
    body = WT::FunctionBody.new(
      [WT::VALUE_TYPE[:i32], WT::VALUE_TYPE[:i32], WT::VALUE_TYPE[:i64]],
      [0x0B].pack("C*")
    )

    encoded = WME.send(:encode_function_body, body)

    assert_operator encoded.bytesize, :>, body.code.bytesize
  end

  def test_rejects_function_import_without_type_index
    wasm_module = WT::WasmModule.new
    wasm_module.types << WT::FuncType.new([], [])
    wasm_module.imports << WT::Import.new("env", "f", WT::EXTERNAL_KIND[:function], WT::MemoryType.new(WT::Limits.new(1, nil)))

    error = assert_raises(WME::WasmEncodeError) do
      WME.encode_module(wasm_module)
    end

    assert_match(/type index/, error.message)
  end

  def test_encodes_table_global_and_start_sections
    wasm_module = WT::WasmModule.new
    wasm_module.types << WT::FuncType.new([], [WT::VALUE_TYPE[:i32]])
    wasm_module.functions << 0
    wasm_module.tables << WT::TableType.new(WT::FUNCREF, WT::Limits.new(1, nil))
    wasm_module.globals << WT::Global.new(
      WT::GlobalType.new(WT::VALUE_TYPE[:i32], false),
      [0x41, 0x2A, 0x0B].pack("C*")
    )
    wasm_module.start = 0
    wasm_module.code << WT::FunctionBody.new([], [0x41, 0x00, 0x0B].pack("C*"))

    encoded = WME.encode_module(wasm_module)

    assert_operator encoded.bytesize, :>, 8
    assert_includes encoded.bytes, 0x08
  end
end
