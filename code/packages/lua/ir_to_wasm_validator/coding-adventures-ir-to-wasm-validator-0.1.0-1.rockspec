package = "coding-adventures-ir-to-wasm-validator"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Validate whether Lua compiler IR can be lowered into the current WASM backend",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-ir-to-wasm-compiler >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.ir_to_wasm_validator"] = "src/coding_adventures/ir_to_wasm_validator/init.lua",
    },
}
