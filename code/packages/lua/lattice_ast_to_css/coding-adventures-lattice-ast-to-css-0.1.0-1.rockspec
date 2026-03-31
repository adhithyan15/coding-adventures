package = "coding-adventures-lattice-ast-to-css"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "Lattice AST → CSS compiler — walks a Lattice AST and emits CSS text",
  homepage = "https://github.com/example/coding-adventures",
  license  = "MIT",
}
dependencies = {
  "lua >= 5.1",
  "coding-adventures-lattice-parser",
}
build = {
  type    = "builtin",
  modules = {
    ["coding_adventures.lattice_ast_to_css"] =
      "src/coding_adventures/lattice_ast_to_css/init.lua",
  },
}
