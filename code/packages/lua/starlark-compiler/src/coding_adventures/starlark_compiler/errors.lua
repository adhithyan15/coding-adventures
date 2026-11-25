-- ============================================================================
-- Closed error hierarchy for Starlark opcode provider.
--
-- Standing Rule #20: All error instances are created through closed constructors
-- with static, payload-blind messages from messages.lua.
-- ============================================================================

local messages = require("coding_adventures.starlark_compiler.messages")

local StarlarkCompilerError = {}
StarlarkCompilerError.__index = StarlarkCompilerError

function StarlarkCompilerError:__tostring()
  return self.message
end

local function make_error(error_id, message)
  local err = {
    error_id = error_id,
    message = message,
  }
  setmetatable(err, StarlarkCompilerError)
  return err
end

local errors = {
  StarlarkCompilerError = StarlarkCompilerError,

  invalid_byte = function()
    return make_error(messages.ERROR_ID_INVALID_BYTE, messages.MSG_INVALID_BYTE)
  end,

  unknown_opcode = function()
    return make_error(messages.ERROR_ID_UNKNOWN_OPCODE, messages.MSG_UNKNOWN_OPCODE)
  end,

  unknown_operator = function()
    return make_error(messages.ERROR_ID_UNKNOWN_OPERATOR, messages.MSG_UNKNOWN_OPERATOR)
  end,

  corpus_byte_limit_exceeded = function()
    return make_error(messages.ERROR_ID_INVALID_CORPUS, messages.MSG_CORPUS_BYTE_LIMIT_EXCEEDED)
  end,

  corpus_malformed_transport = function()
    return make_error(messages.ERROR_ID_INVALID_CORPUS, messages.MSG_CORPUS_MALFORMED_TRANSPORT)
  end,

  corpus_schema_violation = function()
    return make_error(messages.ERROR_ID_INVALID_CORPUS, messages.MSG_CORPUS_SCHEMA_VIOLATION)
  end,

  is_starlark_compiler_error = function(err)
    if type(err) ~= "table" then
      return false
    end
    return getmetatable(err) == StarlarkCompilerError
  end,
}

return errors
