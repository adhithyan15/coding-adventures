package = "coding-adventures-wasm-simulator"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "WebAssembly interpreter/simulator",
  homepage = "https://github.com/example/coding-adventures",
  license  = "MIT",
}
dependencies = {
  "lua >= 5.4",
  "coding-adventures-wasm-leb128",
  "coding-adventures-wasm-types",
  "coding-adventures-wasm-opcodes",
  "coding-adventures-wasm-module-parser",
}
build = {
  type    = "builtin",
  modules = { ["coding_adventures.wasm_simulator"] = "src/coding_adventures/wasm_simulator/init.lua" },
}
