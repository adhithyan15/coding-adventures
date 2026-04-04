-- mosaic-emit-react — React backend: emits TSX functional components from MosaicIR
--
-- This module is part of the coding-adventures project, an educational
-- computing stack built from logic gates up through interpreters.
----
-- Usage:
--
--   local m = require("coding_adventures.mosaic_emit_react")
--
-- ============================================================================

local mosaic_vm = require("coding_adventures.mosaic_vm")
local mosaic_analyzer = require("coding_adventures.mosaic_analyzer")
local mosaic_parser = require("coding_adventures.mosaic_parser")
local mosaic_lexer = require("coding_adventures.mosaic_lexer")
local grammar_tools = require("coding_adventures.grammar_tools")
local lexer = require("coding_adventures.lexer")
local directed_graph = require("coding_adventures.directed_graph")
local parser = require("coding_adventures.parser")
local state_machine = require("coding_adventures.state_machine")

local M = {}

M.VERSION = "0.1.0"

-- TODO: Implement mosaic-emit-react

return M
