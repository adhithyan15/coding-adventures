-- This script intentionally does not add monorepo source directories to
-- package.path. BUILD runs it after LuaRocks installation so it exercises the
-- deployed compiler module.
local lattice_ast_to_css = require("coding_adventures.lattice_ast_to_css")

local css = lattice_ast_to_css.compile({
    rule_name = "stylesheet",
    children = {},
})
assert(css == "", "installed compiler did not compile an empty stylesheet")

print("installed lattice_ast_to_css runtime: ok")
