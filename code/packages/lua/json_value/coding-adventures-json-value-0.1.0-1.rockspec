package = "coding-adventures-json-value"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "JSON value evaluator — AST to native Lua values and back",
  homepage = "https://github.com/example/coding-adventures",
  license  = "MIT",
}
dependencies = {
  "lua >= 5.1",
  "coding-adventures-json-parser",
}
build = {
  type    = "builtin",
  modules = { ["coding_adventures.json_value"] = "src/coding_adventures/json_value/init.lua" },
}
