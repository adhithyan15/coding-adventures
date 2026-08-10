-- This script intentionally does not add monorepo source directories to
-- package.path. BUILD runs it after LuaRocks installation so it exercises the
-- deployed module and its bundled grammar data.
local lattice_parser = require("coding_adventures.lattice_parser")

local ast = lattice_parser.parse(".installed { color: green; }")
assert(type(ast) == "table", "installed parser did not return an AST")
assert(ast.rule_name == "stylesheet", "installed parser returned the wrong root")

print("installed lattice_parser runtime: ok")
