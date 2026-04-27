package = "coding-adventures-ir-to-wasm-compiler"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Lower generic compiler IR into parser-compatible WebAssembly module tables",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-compiler-ir >= 0.1.0",
    "coding-adventures-wasm-leb128 >= 0.1.0",
    "coding-adventures-wasm-opcodes >= 0.1.0",
    "coding-adventures-wasm-types >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.ir_to_wasm_compiler"] = "src/coding_adventures/ir_to_wasm_compiler/init.lua",
    },
}
