-- ============================================================================
-- coding_adventures.canonical_cbor.messages
-- ============================================================================
-- Single source of truth for all 14 closed payload-blind CBR01 error diagnostics,
-- immutability failure diagnostics, and security redaction placeholders
-- (Standing Rule #20).
--
-- No user-supplied data, keys, values, offsets, or sizes are ever reflected in
-- error diagnostics, guaranteeing defense-in-depth against secret leakage.
-- ============================================================================

local M = {}

-- Constant placeholder used when stringifying spanned entries to avoid leaking
-- sensitive keys or values in logs or exception contexts.
M.REDACTED_PLACEHOLDER = "<redacted>"

-- Fixed payload-blind diagnostic when an invalid error ID is requested.
M.UNKNOWN_ERROR_ID_MESSAGE = "canonical-cbor: unknown error id"

-- Fixed payload-blind diagnostics for immutability violations.
M.IMMUTABLE_OBJECT_MESSAGE = "canonical-cbor: immutable object cannot be modified"
M.READ_ONLY_MODULE_MESSAGE = "canonical-cbor: module table is read-only"
M.IMMUTABLE_ERROR_MESSAGES = "canonical-cbor: ERROR_MESSAGES table is immutable"

local ERROR_MESSAGES = {
  ["unexpected-eof"] = "canonical-cbor: unexpected end of input",
  ["trailing-bytes"] = "canonical-cbor: trailing bytes after decoded item",
  ["reserved"] = "canonical-cbor: reserved additional-info value",
  ["indefinite"] = "canonical-cbor: indefinite item rejected",
  ["non-minimal-integer"] = "canonical-cbor: argument is not in smallest form",
  ["invalid-utf8"] = "canonical-cbor: text is not valid UTF-8",
  ["non-canonical-map-order"] = "canonical-cbor: map key order is not canonical",
  ["unsupported-simple"] = "canonical-cbor: unsupported simple value",
  ["float-not-supported"] = "canonical-cbor: floats are not supported",
  ["too-deep"] = "canonical-cbor: decoded nesting is too deep",
  ["length-too-large"] = "canonical-cbor: declared length is too large",
  ["duplicate-map-key"] = "canonical-cbor: duplicate canonical map key",
  ["encode-too-deep"] = "canonical-cbor: encoded nesting is too deep",
  ["encode-too-large"] = "canonical-cbor: encoded item is too large",
}

--- Lookup the closed payload-blind diagnostic message for a given error ID.
--- @param id string
--- @return string
function M.message_for(id)
  local msg = ERROR_MESSAGES[id]
  if not msg then
    error(M.UNKNOWN_ERROR_ID_MESSAGE, 0)
  end
  return msg
end

--- Validate whether a string is a recognized CBR01 error ID.
--- @param id string
--- @return boolean
function M.is_valid_id(id)
  return ERROR_MESSAGES[id] ~= nil
end

M.ERROR_MESSAGES = setmetatable({}, {
  __index = ERROR_MESSAGES,
  __newindex = function()
    error(M.IMMUTABLE_ERROR_MESSAGES, 0)
  end,
  __metatable = false,
})

return setmetatable(M, {
  __newindex = function()
    error(M.READ_ONLY_MODULE_MESSAGE, 0)
  end,
  __metatable = false,
})
