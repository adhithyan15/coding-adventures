package = "coding-adventures-jit-core"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "JIT coordinator for LANG VM modules",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-interpreter-ir >= 0.1.0",
    "coding-adventures-vm-core >= 0.1.0",
    "coding-adventures-codegen-core >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.jit_core"] = "src/coding_adventures/jit_core/init.lua",
    },
}
