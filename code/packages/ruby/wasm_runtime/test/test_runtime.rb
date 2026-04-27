# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_runtime"

# ==========================================================================
# Tests for WasmRuntime --- Instantiation and Error Paths
# ==========================================================================
#
# The Runtime composes parse, validate, instantiate, and execute.
# These tests focus on:
#   1. Instantiation of minimal modules
#   2. Export resolution and error handling
#   3. Error paths (missing export, wrong kind, bad function)
#   4. WasmInstance structure
#   5. Module with memory and data segments
#   6. Module with globals
# ==========================================================================

class TestRuntime < Minitest::Test
  WR = CodingAdventures::WasmRuntime
  WE = CodingAdventures::WasmExecution
  WT = CodingAdventures::WasmTypes
  TrapError = CodingAdventures::WasmExecution::TrapError

  def encode_unsigned(value)
    CodingAdventures::WasmLeb128.encode_unsigned(value).bytes
  end

  # Build a minimal WASM module that exports an "add" function.
  # (i32, i32) -> (i32) : local.get 0, local.get 1, i32.add
  def build_add_wasm
    parts = []

    # Header
    parts.push(0x00, 0x61, 0x73, 0x6D)
    parts.push(0x01, 0x00, 0x00, 0x00)

    # Type section: (i32, i32) -> (i32)
    type_payload = [0x01, 0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7F]
    parts.push(0x01)
    parts.concat(encode_unsigned(type_payload.length))
    parts.concat(type_payload)

    # Function section
    func_payload = [0x01, 0x00]
    parts.push(0x03)
    parts.concat(encode_unsigned(func_payload.length))
    parts.concat(func_payload)

    # Export section
    name_bytes = "add".bytes
    export_payload = [
      0x01,
      *encode_unsigned(name_bytes.length),
      *name_bytes,
      0x00, 0x00
    ]
    parts.push(0x07)
    parts.concat(encode_unsigned(export_payload.length))
    parts.concat(export_payload)

    # Code section
    body_code = [0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B]
    body_payload = [0x00, *body_code]
    func_body = [*encode_unsigned(body_payload.length), *body_payload]
    code_payload = [0x01, *func_body]
    parts.push(0x0A)
    parts.concat(encode_unsigned(code_payload.length))
    parts.concat(code_payload)

    parts.pack("C*")
  end

  # Build a WASM module that exports a "get_byte" function which reads
  # from memory: (i32) -> (i32) using i32.load8_u
  def build_memory_wasm
    parts = []

    # Header
    parts.push(0x00, 0x61, 0x73, 0x6D)
    parts.push(0x01, 0x00, 0x00, 0x00)

    # Type section: (i32) -> (i32)
    type_payload = [0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F]
    parts.push(0x01)
    parts.concat(encode_unsigned(type_payload.length))
    parts.concat(type_payload)

    # Function section
    func_payload = [0x01, 0x00]
    parts.push(0x03)
    parts.concat(encode_unsigned(func_payload.length))
    parts.concat(func_payload)

    # Memory section: 1 page min, no max
    mem_payload = [0x01, 0x00, 0x01]
    parts.push(0x05)
    parts.concat(encode_unsigned(mem_payload.length))
    parts.concat(mem_payload)

    # Export section (ID 7)
    name_bytes = "get_byte".bytes
    export_payload = [
      0x01,
      *encode_unsigned(name_bytes.length),
      *name_bytes,
      0x00, 0x00
    ]
    parts.push(0x07)
    parts.concat(encode_unsigned(export_payload.length))
    parts.concat(export_payload)

    # Code section (ID 10) -- must come before data section (ID 11)
    body_code = [0x20, 0x00, 0x2D, 0x00, 0x00, 0x0B]
    body_payload = [0x00, *body_code]
    func_body = [*encode_unsigned(body_payload.length), *body_payload]
    code_payload = [0x01, *func_body]
    parts.push(0x0A)
    parts.concat(encode_unsigned(code_payload.length))
    parts.concat(code_payload)

    # Data section (ID 11): write [0x42] at offset 0
    data_offset_expr = [0x41, 0x00, 0x0B] # i32.const 0, end
    data_bytes = [0x42]
    data_segment = [
      0x00,
      *data_offset_expr,
      *encode_unsigned(data_bytes.length),
      *data_bytes
    ]
    data_payload = [0x01, *data_segment]
    parts.push(0x0B)
    parts.concat(encode_unsigned(data_payload.length))
    parts.concat(data_payload)

    parts.pack("C*")
  end

  # Build minimal square.wasm (reused from existing test)
  def build_square_wasm
    parts = []
    parts.push(0x00, 0x61, 0x73, 0x6D)
    parts.push(0x01, 0x00, 0x00, 0x00)

    type_payload = [0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F]
    parts.push(0x01)
    parts.concat(encode_unsigned(type_payload.length))
    parts.concat(type_payload)

    func_payload = [0x01, 0x00]
    parts.push(0x03)
    parts.concat(encode_unsigned(func_payload.length))
    parts.concat(func_payload)

    name_bytes = "square".bytes
    export_payload = [
      0x01,
      *encode_unsigned(name_bytes.length),
      *name_bytes,
      0x00, 0x00
    ]
    parts.push(0x07)
    parts.concat(encode_unsigned(export_payload.length))
    parts.concat(export_payload)

    body_code = [0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B]
    body_payload = [0x00, *body_code]
    func_body = [*encode_unsigned(body_payload.length), *body_payload]
    code_payload = [0x01, *func_body]
    parts.push(0x0A)
    parts.concat(encode_unsigned(code_payload.length))
    parts.concat(code_payload)

    parts.pack("C*")
  end

  # Build a function that branches to a block end and then continues with
  # more instructions before the final function end.
  def build_branch_to_end_then_continue_wasm
    parts = []
    parts.push(0x00, 0x61, 0x73, 0x6D)
    parts.push(0x01, 0x00, 0x00, 0x00)

    # Type section: () -> (i32)
    type_payload = [0x01, 0x60, 0x00, 0x01, 0x7F]
    parts.push(0x01)
    parts.concat(encode_unsigned(type_payload.length))
    parts.concat(type_payload)

    # Function section
    func_payload = [0x01, 0x00]
    parts.push(0x03)
    parts.concat(encode_unsigned(func_payload.length))
    parts.concat(func_payload)

    # Export section
    name_bytes = "branch_then_continue".bytes
    export_payload = [
      0x01,
      *encode_unsigned(name_bytes.length),
      *name_bytes,
      0x00, 0x00
    ]
    parts.push(0x07)
    parts.concat(encode_unsigned(export_payload.length))
    parts.concat(export_payload)

    # Code:
    #   block
    #     i32.const 1
    #     br_if 0
    #     unreachable
    #   end
    #   i32.const 7
    #   end
    body_code = [0x02, 0x40, 0x41, 0x01, 0x0D, 0x00, 0x00, 0x0B, 0x41, 0x07, 0x0B]
    body_payload = [0x00, *body_code]
    func_body = [*encode_unsigned(body_payload.length), *body_payload]
    code_payload = [0x01, *func_body]
    parts.push(0x0A)
    parts.concat(encode_unsigned(code_payload.length))
    parts.concat(code_payload)

    parts.pack("C*")
  end

  # ── Load and Parse ─────────────────────────────────────────────────

  def test_load_returns_wasm_module
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    assert_kind_of WT::WasmModule, mod
  end

  def test_load_from_byte_array
    runtime = WR::Runtime.new
    bytes = build_add_wasm.bytes
    mod = runtime.load(bytes)
    assert_kind_of WT::WasmModule, mod
  end

  # ── Validate ───────────────────────────────────────────────────────

  def test_validate_returns_validated_module
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    validated = runtime.validate(mod)
    refute_nil validated
  end

  # ── Instantiate ────────────────────────────────────────────────────

  def test_instantiate_returns_wasm_instance
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    instance = runtime.instantiate(mod)
    assert_kind_of WR::WasmInstance, instance
  end

  def test_instance_has_exports
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    instance = runtime.instantiate(mod)
    assert instance.exports.key?("add")
  end

  def test_instance_exports_function_kind
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    instance = runtime.instantiate(mod)
    exp = instance.exports["add"]
    assert_includes [:function, WT::EXTERNAL_KIND[:function]], exp[:kind]
  end

  # ── Call Exported Functions ────────────────────────────────────────

  def test_call_add_function
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    instance = runtime.instantiate(mod)
    result = runtime.call(instance, "add", [3, 7])
    assert_equal [10], result
  end

  def test_call_add_with_negative_numbers
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    instance = runtime.instantiate(mod)
    result = runtime.call(instance, "add", [-5, 3])
    assert_equal [-2], result
  end

  def test_call_add_with_zero
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    instance = runtime.instantiate(mod)
    result = runtime.call(instance, "add", [0, 0])
    assert_equal [0], result
  end

  # ── load_and_run Shortcut ──────────────────────────────────────────

  def test_load_and_run_square
    runtime = WR::Runtime.new
    result = runtime.load_and_run(build_square_wasm, "square", [5])
    assert_equal [25], result
  end

  def test_load_and_run_add
    runtime = WR::Runtime.new
    result = runtime.load_and_run(build_add_wasm, "add", [100, 200])
    assert_equal [300], result
  end

  def test_branch_to_non_final_end_continues_execution
    runtime = WR::Runtime.new
    result = runtime.load_and_run(build_branch_to_end_then_continue_wasm, "branch_then_continue", [])
    assert_equal [7], result
  end

  # ── Error Paths ────────────────────────────────────────────────────

  def test_call_missing_export_traps
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    instance = runtime.instantiate(mod)
    assert_raises(TrapError) { runtime.call(instance, "nonexistent", []) }
  end

  def test_load_and_run_missing_entry_traps
    runtime = WR::Runtime.new
    assert_raises(TrapError) do
      runtime.load_and_run(build_add_wasm, "nonexistent", [])
    end
  end

  # ── WasmInstance Structure ─────────────────────────────────────────

  def test_instance_has_func_types
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    instance = runtime.instantiate(mod)
    assert_kind_of Array, instance.func_types
    refute_empty instance.func_types
  end

  def test_instance_has_func_bodies
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    instance = runtime.instantiate(mod)
    assert_kind_of Array, instance.func_bodies
    refute_empty instance.func_bodies
  end

  def test_instance_memory_nil_for_no_memory_module
    runtime = WR::Runtime.new
    mod = runtime.load(build_add_wasm)
    instance = runtime.instantiate(mod)
    assert_nil instance.memory
  end

  # ── Module with Memory ─────────────────────────────────────────────

  def test_module_with_memory_has_memory
    runtime = WR::Runtime.new
    mod = runtime.load(build_memory_wasm)
    instance = runtime.instantiate(mod)
    refute_nil instance.memory
  end

  def test_module_with_data_segment
    runtime = WR::Runtime.new
    mod = runtime.load(build_memory_wasm)
    instance = runtime.instantiate(mod)
    # The data segment wrote 0x42 at offset 0
    result = runtime.call(instance, "get_byte", [0])
    assert_equal [0x42], result
  end

  # ── Runtime with WASI host ─────────────────────────────────────────

  def test_runtime_with_wasi_stub_host
    wasi = WR::WasiStub.new
    runtime = WR::Runtime.new(wasi)
    # Should not raise even with WASI host
    result = runtime.load_and_run(build_square_wasm, "square", [4])
    assert_equal [16], result
  end

  # ── Version ────────────────────────────────────────────────────────

  def test_version_exists
    refute_nil WR::VERSION
  end
end
