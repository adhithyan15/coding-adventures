package = "coding-adventures-lattice-parser"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "Lattice CSS superset parser — builds ASTs from Lattice source",
  homepage = "https://github.com/example/coding-adventures",
  license  = "MIT",
}
dependencies = {
  "lua >= 5.1",
  "coding-adventures-lattice-lexer",
  "coding-adventures-parser",
  "coding-adventures-grammar-tools",
}
build = {
  type    = "builtin",
  modules = { ["coding_adventures.lattice_parser"] = "src/coding_adventures/lattice_parser/init.lua" },
}
