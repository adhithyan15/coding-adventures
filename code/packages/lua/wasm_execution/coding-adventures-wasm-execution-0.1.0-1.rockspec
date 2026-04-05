package = "coding-adventures-wasm-execution"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "WebAssembly 1.0 wasm-execution",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-wasm-leb128 >= 0.1.0",
    "coding-adventures-wasm-types >= 0.1.0",
    "coding-adventures-wasm-opcodes >= 0.1.0",
    "coding-adventures-wasm-module-parser >= 0.1.0",
    "coding-adventures-virtual-machine >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.wasm_execution"] = "src/coding_adventures/wasm_execution/init.lua",
    },
}
