package = "coding-adventures-nib-ir-compiler"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Nib typed AST to generic IR compiler for Lua",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-compiler-ir >= 0.1.0",
    "coding-adventures-nib-type-checker >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.nib_ir_compiler"] = "src/coding_adventures/nib_ir_compiler/init.lua",
    },
}
