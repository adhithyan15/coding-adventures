-- wasm-execution — WebAssembly 1.0 wasm-execution
--
-- This module is part of the coding-adventures project, an educational
-- computing stack built from logic gates up through interpreters.
----
-- Usage:
--
--   local m = require("coding_adventures.wasm_execution")
--
-- ============================================================================

local wasm_leb128 = require("coding_adventures.wasm_leb128")
local wasm_types = require("coding_adventures.wasm_types")
local wasm_opcodes = require("coding_adventures.wasm_opcodes")
local wasm_module_parser = require("coding_adventures.wasm_module_parser")
local virtual_machine = require("coding_adventures.virtual_machine")

local M = {}

M.VERSION = "0.1.0"

-- TODO: Implement wasm-execution

return M
