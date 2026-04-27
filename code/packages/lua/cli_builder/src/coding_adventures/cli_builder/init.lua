-- init.lua -- CLI Builder entry point
-- ===================================

local Errors = require("coding_adventures.cli_builder.errors")
local Types = require("coding_adventures.cli_builder.types")
local SpecLoader = require("coding_adventures.cli_builder.spec_loader")
local TokenClassifier = require("coding_adventures.cli_builder.token_classifier")
local Parser = require("coding_adventures.cli_builder.parser")

local clibuilder = {
    Errors = Errors,
    Types = Types,
    SpecLoader = SpecLoader,
    TokenClassifier = TokenClassifier,
    Parser = Parser
}

--- Create a new Parser instance.
--
-- @param spec_file string Path to the JSON specification.
-- @param argv table The argument vector (including program name at [1]).
-- @return Parser, error The parser instance or nil + error.
function clibuilder.new(spec_file, argv)
    return Parser.new(spec_file, argv)
end

return clibuilder
