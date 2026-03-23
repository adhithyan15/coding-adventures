package = "coding-adventures-virtual-machine"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Stack-based bytecode interpreter with eval loop, value stack, and variable environment",
    detailed = "Two virtual machines: VirtualMachine (hard-coded opcodes) and GenericVM (pluggable handler-based). Both support coroutine-based step-through debugging.",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.virtual_machine"] = "src/coding_adventures/virtual_machine/init.lua",
    },
}
