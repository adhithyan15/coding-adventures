package = "coding-adventures-wasm-module-parser"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "WebAssembly binary module parser",
  homepage = "https://github.com/example/coding-adventures",
  license  = "MIT",
}
dependencies = {
  "lua >= 5.1",
  "coding-adventures-wasm-leb128",
  "coding-adventures-wasm-types",
}
build = {
  type    = "builtin",
  modules = { ["coding_adventures.wasm_module_parser"] = "src/coding_adventures/wasm_module_parser/init.lua" },
}
