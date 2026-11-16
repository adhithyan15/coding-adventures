-- ============================================================================
-- coding_adventures.canonical_cbor
-- ============================================================================
-- RFC 8949 Deterministic Canonical CBOR Codec (CBR01 profile).
--
-- Exposes the public API for encoding, decoding, and constructing strongly typed,
-- immutable canonical CBOR values, with length-first map key ordering (§4.2.3),
-- strict 64-bit unsigned bounds checking, payload-blind diagnostics, and
-- metatable spoofing defenses.
-- ============================================================================

local constants = require("coding_adventures.canonical_cbor.constants")
local messages = require("coding_adventures.canonical_cbor.messages")
local types = require("coding_adventures.canonical_cbor.types")
local codec = require("coding_adventures.canonical_cbor.codec")

local M = {}

-- Constants and messages modules
M.constants = constants
M.messages = messages

-- Type constructors and models
M.CborError = types.CborError
M.Span = types.Span
M.SpannedMapEntry = types.SpannedMapEntry
M.DecodeMapSpansResult = types.DecodeMapSpansResult
M.CborUnsigned = types.CborUnsigned
M.CborNegative = types.CborNegative
M.CborBytes = types.CborBytes
M.CborText = types.CborText
M.CborArray = types.CborArray
M.CborMapEntry = types.CborMapEntry
M.CborMap = types.CborMap
M.CborTag = types.CborTag
M.CborBool = types.CborBool
M.CborNull = types.CborNull

-- Codec API
M.encode_checked = codec.encode_checked
M.encode_into_checked = codec.encode_into_checked
M.encode = codec.encode_checked
M.encode_into = codec.encode_into_checked
M.decode = codec.decode
M.decode_map_spans = codec.decode_map_spans

return setmetatable(M, {
  __newindex = function()
    error(messages.READ_ONLY_MODULE_MESSAGE, 0)
  end,
  __metatable = false,
})
