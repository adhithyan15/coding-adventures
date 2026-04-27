package = "coding-adventures-nib-wasm-compiler"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "End-to-end compiler from Nib source to WebAssembly bytes",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-nib-parser >= 0.1.0",
    "coding-adventures-nib-type-checker >= 0.1.0",
    "coding-adventures-nib-ir-compiler >= 0.1.0",
    "coding-adventures-ir-to-wasm-compiler >= 0.1.0",
    "coding-adventures-ir-to-wasm-validator >= 0.1.0",
    "coding-adventures-wasm-module-encoder >= 0.1.0",
    "coding-adventures-wasm-validator >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.nib_wasm_compiler"] = "src/coding_adventures/nib_wasm_compiler/init.lua",
    },
}
