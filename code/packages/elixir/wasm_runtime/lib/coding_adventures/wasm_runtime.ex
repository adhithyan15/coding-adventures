defmodule CodingAdventures.WasmRuntime do
  @moduledoc """
  WebAssembly 1.0 runtime for Elixir.

  This module re-exports the main runtime API for convenience.
  The full pipeline is: parse -> validate -> instantiate -> execute.

  ## Quick Start

      alias CodingAdventures.WasmRuntime.Runtime
      alias CodingAdventures.WasmExecution.Values

      {:ok, instance} = Runtime.instantiate_bytes(wasm_bytes)
      results = Runtime.call(instance, "add", [Values.i32(3), Values.i32(4)])

  ## Modules

  - `Runtime` -- high-level API (parse + validate + instantiate + call)
  - `Instance` -- a live module instance with memory and globals
  - `WasiHost` -- preferred name for the WASI host surface
  - `WasiStub` -- backward-compatible alias for the WASI host helpers
  """
end
