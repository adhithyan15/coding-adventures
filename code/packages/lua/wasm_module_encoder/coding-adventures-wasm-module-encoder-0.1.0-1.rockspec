package = "coding-adventures-wasm-module-encoder"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Encode Lua WebAssembly module tables into raw .wasm binaries",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-wasm-leb128 >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.wasm_module_encoder"] = "src/coding_adventures/wasm_module_encoder/init.lua",
    },
}
