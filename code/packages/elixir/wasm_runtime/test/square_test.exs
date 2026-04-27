defmodule CodingAdventures.WasmRuntime.SquareTest do
  use ExUnit.Case, async: true

  @moduledoc """
  End-to-end test: compile a "square" function as raw WASM bytecodes,
  parse + validate + instantiate + call it, and verify square(5) == 25.

  This test constructs a minimal valid WASM module by hand (no compiler
  needed). The module exports a single function `square(x: i32) -> i32`
  that computes `x * x` using `local.get 0` twice and `i32.mul`.

  ## WASM Module Structure

      (module
        (type (func (param i32) (result i32)))    ;; type 0
        (func (export "square") (type 0)           ;; func 0
          local.get 0
          local.get 0
          i32.mul
        )
      )

  ## Binary Layout

      Header:  \\x00asm\\x01\\x00\\x00\\x00
      Type section (1):   1 type -> (i32) -> (i32)
      Function section (3): 1 function -> type index 0
      Export section (7):   "square" -> function 0
      Code section (10):    1 body -> [local.get 0, local.get 0, i32.mul, end]
  """

  alias CodingAdventures.WasmExecution.Values

  # Build a minimal WASM module binary by hand
  defp build_square_wasm do
    # -- WASM Header --
    header = <<0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00>>

    # -- Type Section (ID=1) --
    # One function type: (i32) -> (i32)
    #   0x60 = func type tag
    #   0x01 0x7F = 1 param, i32
    #   0x01 0x7F = 1 result, i32
    type_entry = <<0x60, 0x01, 0x7F, 0x01, 0x7F>>
    # count = 1
    type_section_body = <<0x01>> <> type_entry
    type_section = <<0x01>> <> leb128_size(type_section_body) <> type_section_body

    # -- Function Section (ID=3) --
    # One function referencing type index 0
    # count = 1, type_idx = 0
    func_section_body = <<0x01, 0x00>>
    func_section = <<0x03>> <> leb128_size(func_section_body) <> func_section_body

    # -- Export Section (ID=7) --
    # Export "square" as function index 0
    export_name = "square"
    export_entry = leb128_size(export_name) <> export_name <> <<0x00, 0x00>>
    # count = 1
    export_section_body = <<0x01>> <> export_entry
    export_section = <<0x07>> <> leb128_size(export_section_body) <> export_section_body

    # -- Code Section (ID=10) --
    # One function body:
    #   locals: 0 local declarations
    #   code: local.get 0, local.get 0, i32.mul, end
    func_code = <<0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B>>
    # 0 local decl groups + code
    func_body = <<0x00>> <> func_code
    func_body_with_size = leb128_size(func_body) <> func_body
    # count = 1
    code_section_body = <<0x01>> <> func_body_with_size
    code_section = <<0x0A>> <> leb128_size(code_section_body) <> code_section_body

    header <> type_section <> func_section <> export_section <> code_section
  end

  # Encode a byte count as a single-byte LEB128 (sufficient for small values)
  defp leb128_size(data) when is_binary(data), do: <<byte_size(data)>>

  test "square(5) returns 25" do
    wasm_bytes = build_square_wasm()

    # Parse
    {:ok, wasm_module} = CodingAdventures.WasmModuleParser.parse(wasm_bytes)
    assert length(wasm_module.types) == 1
    assert length(wasm_module.functions) == 1
    assert length(wasm_module.code) == 1

    # Validate
    {:ok, validated} = CodingAdventures.WasmValidator.validate(wasm_module)
    assert length(validated.func_types) == 1

    # Instantiate
    {:ok, instance} = CodingAdventures.WasmRuntime.Instance.from_validated(validated)

    # Call square(5)
    results = CodingAdventures.WasmRuntime.Instance.call(instance, "square", [Values.i32(5)])

    assert length(results) == 1
    [result] = results
    # i32
    assert result.type == 0x7F
    assert result.value == 25
  end

  test "square(0) returns 0" do
    wasm_bytes = build_square_wasm()
    {:ok, wasm_module} = CodingAdventures.WasmModuleParser.parse(wasm_bytes)
    {:ok, validated} = CodingAdventures.WasmValidator.validate(wasm_module)
    {:ok, instance} = CodingAdventures.WasmRuntime.Instance.from_validated(validated)

    [result] = CodingAdventures.WasmRuntime.Instance.call(instance, "square", [Values.i32(0)])
    assert result.value == 0
  end

  test "square(-3) returns 9" do
    wasm_bytes = build_square_wasm()
    {:ok, wasm_module} = CodingAdventures.WasmModuleParser.parse(wasm_bytes)
    {:ok, validated} = CodingAdventures.WasmValidator.validate(wasm_module)
    {:ok, instance} = CodingAdventures.WasmRuntime.Instance.from_validated(validated)

    [result] = CodingAdventures.WasmRuntime.Instance.call(instance, "square", [Values.i32(-3)])
    assert result.value == 9
  end

  test "Runtime.instantiate_bytes convenience API" do
    wasm_bytes = build_square_wasm()
    {:ok, instance} = CodingAdventures.WasmRuntime.Runtime.instantiate_bytes(wasm_bytes)

    results = CodingAdventures.WasmRuntime.Runtime.call(instance, "square", [Values.i32(7)])
    assert [%{value: 49}] = results
  end
end
