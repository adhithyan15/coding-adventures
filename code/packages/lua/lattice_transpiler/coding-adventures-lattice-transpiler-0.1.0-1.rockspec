package = "coding-adventures-lattice-transpiler"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "Lattice CSS superset transpiler — compiles Lattice to CSS",
  homepage = "https://github.com/example/coding-adventures",
  license  = "MIT",
}
dependencies = {
  "lua >= 5.1",
  "coding-adventures-lattice-ast-to-css",
}
build = {
  type    = "builtin",
  modules = {
    ["coding_adventures.lattice_transpiler"] =
      "src/coding_adventures/lattice_transpiler/init.lua",
  },
}
