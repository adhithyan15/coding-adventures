-- ============================================================================
-- Constants for Starlark opcode provider conforming to STP01.
--
-- Standing Rule #20: All magic numbers, limits, and configuration values are
-- isolated in this module.
-- ============================================================================

local constants = {
  PACKAGE_VERSION = "0.1.0",
  STP01_SCHEMA_VERSION = 1,
  STP01_PROFILE = "standalone-provider-v1",
  MAX_CORPUS_BYTES = 131072,
  MAX_JSON_NESTING_DEPTH = 8,
  MIN_BYTE_VALUE = 0,
  MAX_BYTE_VALUE = 255,
  EXPECTED_OPCODE_COUNT = 61,
  EXPECTED_DECODE_COUNT = 256,
  EXPECTED_OPERATOR_COUNT = 34,
  EXPECTED_UNKNOWN_LOOKUP_COUNT = 8,
  EXPECTED_HOLE_COUNT = 195,
  MIN_SURROGATE_CODE_POINT = 0xD800,
  MAX_SURROGATE_CODE_POINT = 0xDFFF,
}

return constants
