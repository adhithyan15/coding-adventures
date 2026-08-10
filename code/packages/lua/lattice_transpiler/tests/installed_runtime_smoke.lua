-- This script intentionally does not add monorepo source directories to
-- package.path. BUILD runs it after LuaRocks installation so it exercises the
-- complete deployed lexer -> parser -> compiler pipeline.
local lattice_transpiler = require("coding_adventures.lattice_transpiler")

local css, err = lattice_transpiler.transpile(".installed { color: green; }")
assert(css, err)
assert(css:find("color: green", 1, true), css)

print("installed lattice_transpiler runtime: ok")
