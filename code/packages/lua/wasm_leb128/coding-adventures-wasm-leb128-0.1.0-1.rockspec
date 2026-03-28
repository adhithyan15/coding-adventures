package = "coding-adventures-wasm-leb128"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "LEB128 variable-length integer encoding for WebAssembly binary format",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.wasm_leb128"] = "src/coding_adventures/wasm_leb128/init.lua",
    },
}
