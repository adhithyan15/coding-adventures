package = "coding-adventures-brainfuck-wasm-compiler"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "End-to-end compiler from Brainfuck source to WebAssembly bytes",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-brainfuck >= 0.1.0",
    "coding-adventures-brainfuck-ir-compiler >= 0.1.0",
    "coding-adventures-ir-to-wasm-compiler >= 0.1.0",
    "coding-adventures-ir-to-wasm-validator >= 0.1.0",
    "coding-adventures-wasm-module-encoder >= 0.1.0",
    "coding-adventures-wasm-validator >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.brainfuck_wasm_compiler"] = "src/coding_adventures/brainfuck_wasm_compiler/init.lua",
    },
}
