-- This script intentionally does not add monorepo source directories to
-- package.path. BUILD runs it after LuaRocks installation so it exercises the
-- deployed JSON lexer, parser, grammar payloads, and value evaluator.
local json_value = require("coding_adventures.json_value")

local value = json_value.from_string('{"items":[1,null,true],"name":"parity"}')
assert(value.name == "parity", "installed evaluator lost the object string")
assert(value.items[1] == 1, "installed evaluator lost the array number")
assert(json_value.is_null(value.items[2]), "installed evaluator lost JSON null")
assert(value.items[3] == true, "installed evaluator lost the array boolean")
assert(
    json_value.to_json(value) == '{"items":[1,null,true],"name":"parity"}',
    "installed evaluator did not serialize deterministically"
)

print("installed json_value runtime: ok")
