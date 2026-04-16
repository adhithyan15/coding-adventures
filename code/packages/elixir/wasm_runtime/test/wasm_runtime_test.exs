defmodule CodingAdventures.WasmRuntimeTest do
  use ExUnit.Case

  alias CodingAdventures.WasmRuntime.Engine
  alias CodingAdventures.WasmValidator

  alias CodingAdventures.WasmTypes.{
    Export,
    FuncType,
    FunctionBody,
    WasmModule
  }

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.WasmRuntime)
  end

  test "loop backedges preserve the loop label across iterations" do
    module = %WasmModule{
      types: [
        %FuncType{params: [], results: [:i32]}
      ],
      functions: [0],
      exports: [
        %Export{name: "countdown", kind: :function, index: 0}
      ],
      code: [
        %FunctionBody{
          locals: [:i32],
          code:
            <<0x41, 0x04, 0x21, 0x00, 0x02, 0x40, 0x03, 0x40, 0x20, 0x00, 0x45, 0x0D, 0x01, 0x20,
              0x00, 0x41, 0x01, 0x6B, 0x21, 0x00, 0x0C, 0x00, 0x0B, 0x0B, 0x20, 0x00, 0x0B>>
        }
      ]
    }

    assert {:ok, validated} = WasmValidator.validate(module)
    {vm, ctx} = Engine.instantiate_full(validated)
    assert [%{value: 0}] = Engine.call_function(vm, ctx, "countdown", [])
  end
end
