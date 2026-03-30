package = "coding-adventures-wasm-opcodes"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "WebAssembly opcode definitions and lookup table",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-wasm-leb128 >= 0.1.0",
    "coding-adventures-wasm-types >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.wasm_opcodes"] = "src/coding_adventures/wasm_opcodes/init.lua",
    },
}
