package = "coding-adventures-dartmouth-basic-ir-compiler"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Dartmouth BASIC compiler on the Lua LANG VM",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-interpreter-ir >= 0.1.0",
    "coding-adventures-vm-core >= 0.1.0",
    "coding-adventures-codegen-core >= 0.1.0",
    "coding-adventures-jit-core >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.dartmouth_basic_ir_compiler"] = "src/coding_adventures/dartmouth_basic_ir_compiler/init.lua",
    },
}
