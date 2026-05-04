package = "coding-adventures-vm-core"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pure Lua LANG VM executor with builtins, memory, metrics, and JIT hooks",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-interpreter-ir >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.vm_core"] = "src/coding_adventures/vm_core/init.lua",
    },
}
