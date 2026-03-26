-- document_html_sanitizer / init.lua
-- =====================================
--
-- Public entry point for the coding_adventures.document_html_sanitizer module.
-- Re-exports the sanitize_html() function and all named policy presets.
--
-- Usage:
--
--   local html_san = require("coding_adventures.document_html_sanitizer")
--
--   local clean = html_san.sanitize_html(raw_html, html_san.HTML_STRICT)
--
-- @module coding_adventures.document_html_sanitizer

local policy_mod = require("coding_adventures.document_html_sanitizer.policy")
local san_mod    = require("coding_adventures.document_html_sanitizer.html_sanitizer")
local url_mod    = require("coding_adventures.document_html_sanitizer.url_utils")

local M = {}

-- ─── Re-exported policy presets ───────────────────────────────────────────────

--- HTML_STRICT — for untrusted HTML from external sources.
M.HTML_STRICT = policy_mod.HTML_STRICT

--- HTML_RELAXED — for authenticated users / internal tools.
M.HTML_RELAXED = policy_mod.HTML_RELAXED

--- HTML_PASSTHROUGH — no sanitization.
M.HTML_PASSTHROUGH = policy_mod.HTML_PASSTHROUGH

-- ─── Re-exported functions ────────────────────────────────────────────────────

--- Sanitize an HTML string by stripping dangerous elements and attributes.
-- @see coding_adventures.document_html_sanitizer.html_sanitizer
M.sanitize_html = san_mod.sanitize_html

-- ─── URL utilities (exposed for testing and advanced use) ─────────────────────

M.strip_control_chars = url_mod.strip_control_chars
M.extract_scheme      = url_mod.extract_scheme
M.is_scheme_allowed   = url_mod.is_scheme_allowed

M.VERSION = "0.1.0"

return M
