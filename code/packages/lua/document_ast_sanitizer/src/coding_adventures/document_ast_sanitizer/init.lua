-- document_ast_sanitizer / init.lua
-- ====================================
--
-- Public entry point for the coding_adventures.document_ast_sanitizer module.
-- Re-exports the sanitize() function and all named policy presets so callers
-- can use a single require:
--
--   local sanitizer = require("coding_adventures.document_ast_sanitizer")
--
--   local safe_doc = sanitizer.sanitize(doc, sanitizer.STRICT)
--
-- @module coding_adventures.document_ast_sanitizer

local policy_mod    = require("coding_adventures.document_ast_sanitizer.policy")
local sanitizer_mod = require("coding_adventures.document_ast_sanitizer.sanitizer")
local url_mod       = require("coding_adventures.document_ast_sanitizer.url_utils")

local M = {}

-- ─── Re-exported policy presets ───────────────────────────────────────────────

--- STRICT preset — for user-generated content.
M.STRICT = policy_mod.STRICT

--- RELAXED preset — for semi-trusted content.
M.RELAXED = policy_mod.RELAXED

--- PASSTHROUGH preset — for fully trusted content.
M.PASSTHROUGH = policy_mod.PASSTHROUGH

--- Build a policy by merging overrides on top of PASSTHROUGH defaults.
M.with_defaults = policy_mod.with_defaults

-- ─── Re-exported functions ────────────────────────────────────────────────────

--- Sanitize a DocumentNode according to a SanitizationPolicy.
-- @see coding_adventures.document_ast_sanitizer.sanitizer
M.sanitize = sanitizer_mod.sanitize

-- ─── URL utilities (exposed for testing and advanced use) ─────────────────────

M.strip_control_chars = url_mod.strip_control_chars
M.extract_scheme      = url_mod.extract_scheme
M.is_scheme_allowed   = url_mod.is_scheme_allowed

M.VERSION = "0.1.0"

return M
