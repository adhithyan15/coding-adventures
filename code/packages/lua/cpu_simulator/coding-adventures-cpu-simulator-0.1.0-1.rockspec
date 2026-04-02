package = "coding-adventures-cpu-simulator"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "CPU simulator building blocks: Memory, SparseMemory, RegisterFile",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.cpu_simulator"]               = "src/coding_adventures/cpu_simulator/init.lua",
        ["coding_adventures.cpu_simulator.memory"]        = "src/coding_adventures/cpu_simulator/memory.lua",
        ["coding_adventures.cpu_simulator.register_file"] = "src/coding_adventures/cpu_simulator/register_file.lua",
    },
}
