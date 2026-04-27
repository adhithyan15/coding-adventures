defmodule CodingAdventures.WasmModuleEncoderTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.WasmModuleEncoder
  alias CodingAdventures.WasmModuleParser

  alias CodingAdventures.WasmTypes.{
    Export,
    FuncType,
    FunctionBody,
    WasmModule
  }

  test "encodes and roundtrips a minimal function module" do
    module = %WasmModule{
      types: [
        %FuncType{params: [:i32], results: [:i32]}
      ],
      functions: [0],
      exports: [
        %Export{name: "id", kind: :function, index: 0}
      ],
      code: [
        %FunctionBody{locals: [], code: <<0x20, 0x00, 0x0B>>}
      ]
    }

    binary = WasmModuleEncoder.encode_module(module)
    assert binary_part(binary, 0, 4) == <<0x00, 0x61, 0x73, 0x6D>>

    assert {:ok, parsed} = WasmModuleParser.parse(binary)
    assert length(parsed.types) == 1
    assert length(parsed.functions) == 1
    assert length(parsed.exports) == 1
    assert length(parsed.code) == 1
  end
end
