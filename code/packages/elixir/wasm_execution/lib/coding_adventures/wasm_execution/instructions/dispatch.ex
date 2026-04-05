defmodule CodingAdventures.WasmExecution.Instructions.Dispatch do
  @moduledoc """
  Central dispatcher that registers ALL WASM instruction handlers on a GenericVM.

  This module composes all instruction handler modules into a single
  `register_all/1` function. It is the single point of contact between
  the engine and the instruction set.

  ## Handler Modules

      ┌────────────────────────────────────────────────────────┐
      │                     Dispatch                            │
      │                                                         │
      │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
      │  │ NumericI32   │  │ NumericI64   │  │ NumericF32   │ │
      │  │ 33 opcodes   │  │ 30 opcodes   │  │ 21 opcodes   │ │
      │  └──────────────┘  └──────────────┘  └──────────────┘ │
      │                                                         │
      │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
      │  │ NumericF64   │  │ Conversion   │  │  Control     │ │
      │  │ 21 opcodes   │  │ 25 opcodes   │  │ 11 opcodes   │ │
      │  └──────────────┘  └──────────────┘  └──────────────┘ │
      │                                                         │
      │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
      │  │ Parametric   │  │  Variable    │  │   Memory     │ │
      │  │  2 opcodes   │  │  5 opcodes   │  │ 27 opcodes   │ │
      │  └──────────────┘  └──────────────┘  └──────────────┘ │
      └────────────────────────────────────────────────────────┘
  """

  alias CodingAdventures.WasmExecution.Instructions.{
    NumericI32,
    NumericI64,
    NumericF32,
    NumericF64,
    Conversion,
    Control,
    Parametric,
    Variable,
    Memory
  }

  @doc """
  Register all WASM instruction handlers on the given GenericVM.

  This is the only function the engine needs to call. It pipes the VM
  through each handler module's `register/1` function, building up the
  complete instruction set.

  ## Example

      vm = GenericVM.new()
      vm = Dispatch.register_all(vm)
      # vm now has handlers for all ~175 WASM 1.0 opcodes
  """
  def register_all(vm) do
    vm
    |> Control.register()
    |> Parametric.register()
    |> Variable.register()
    |> Memory.register()
    |> NumericI32.register()
    |> NumericI64.register()
    |> NumericF32.register()
    |> NumericF64.register()
    |> Conversion.register()
  end
end
