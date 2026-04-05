# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_wasm_leb128"
require "coding_adventures_wasm_types"
require "coding_adventures_wasm_opcodes"
require "coding_adventures_wasm_module_parser"
require "coding_adventures_virtual_machine"

require_relative "coding_adventures/wasm_execution/version"
require_relative "coding_adventures/wasm_execution/trap_error"
require_relative "coding_adventures/wasm_execution/values"
require_relative "coding_adventures/wasm_execution/linear_memory"
require_relative "coding_adventures/wasm_execution/table"
require_relative "coding_adventures/wasm_execution/host_interface"
require_relative "coding_adventures/wasm_execution/decoder"
require_relative "coding_adventures/wasm_execution/const_expr"
require_relative "coding_adventures/wasm_execution/instructions/numeric_i32"
require_relative "coding_adventures/wasm_execution/instructions/numeric_i64"
require_relative "coding_adventures/wasm_execution/instructions/numeric_f32"
require_relative "coding_adventures/wasm_execution/instructions/numeric_f64"
require_relative "coding_adventures/wasm_execution/instructions/conversion"
require_relative "coding_adventures/wasm_execution/instructions/variable"
require_relative "coding_adventures/wasm_execution/instructions/parametric"
require_relative "coding_adventures/wasm_execution/instructions/memory"
require_relative "coding_adventures/wasm_execution/instructions/control"
require_relative "coding_adventures/wasm_execution/instructions/dispatch"
require_relative "coding_adventures/wasm_execution/wasm_execution_engine"

module CodingAdventures
  # WebAssembly 1.0 execution engine --- interprets validated WASM modules.
  module WasmExecution
  end
end
