package = "coding-adventures-interpreter-ir"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Typed LANG VM interpreter IR for Lua packages",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.interpreter_ir"] = "src/coding_adventures/interpreter_ir/init.lua",
    },
}
