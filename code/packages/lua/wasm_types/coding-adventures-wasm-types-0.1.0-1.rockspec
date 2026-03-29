package = "coding-adventures-wasm-types"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "WebAssembly value types and fundamental type definitions",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-wasm-leb128 >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.wasm_types"] = "src/coding_adventures/wasm_types/init.lua",
    },
}
