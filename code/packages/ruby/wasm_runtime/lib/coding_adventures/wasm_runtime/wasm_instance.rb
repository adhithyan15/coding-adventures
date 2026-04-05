# frozen_string_literal: true

# ==========================================================================
# WasmInstance --- A Runtime Instance of a WASM Module
# ==========================================================================
#
# A WASM module is a static artifact (types, functions, layout). An
# instance is a module "brought to life": memory allocated, tables
# created, globals initialized, imports resolved.
# ==========================================================================

module CodingAdventures
  module WasmRuntime
    # A live, executable instance of a WASM module.
    WasmInstance = Struct.new(
      :wasm_module, :memory, :tables, :globals, :global_types,
      :func_types, :func_bodies, :host_functions, :exports, :host,
      keyword_init: true
    )
  end
end
