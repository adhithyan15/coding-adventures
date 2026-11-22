-- ============================================================================
-- Main module entrypoint for coding_adventures.starlark_compiler in Lua.
-- ============================================================================

local constants = require("coding_adventures.starlark_compiler.constants")
local messages = require("coding_adventures.starlark_compiler.messages")
local errors = require("coding_adventures.starlark_compiler.errors")
local opcodes = require("coding_adventures.starlark_compiler.opcodes")

local starlark_compiler = {
  constants = constants,
  messages = messages,
  errors = errors,
  opcodes = opcodes,
}

for k, v in pairs(constants) do
  starlark_compiler[k] = v
end

for k, v in pairs(messages) do
  starlark_compiler[k] = v
end

for k, v in pairs(errors) do
  starlark_compiler[k] = v
end

for k, v in pairs(opcodes) do
  starlark_compiler[k] = v
end

return starlark_compiler
