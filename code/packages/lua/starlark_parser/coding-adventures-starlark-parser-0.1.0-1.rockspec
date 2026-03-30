package = "coding-adventures-starlark-parser"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "Starlark source parser — builds ASTs from Starlark configuration files",
  homepage = "https://github.com/example/coding-adventures",
  license  = "MIT",
}
dependencies = {
  "lua >= 5.1",
  "coding-adventures-starlark-lexer",
  "coding-adventures-parser",
  "coding-adventures-grammar-tools",
}
build = {
  type    = "builtin",
  modules = { ["coding_adventures.starlark_parser"] = "src/coding_adventures/starlark_parser/init.lua" },
}
