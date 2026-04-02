-- coding_adventures.cpu_simulator — module entry point

local memory_mod    = require("coding_adventures.cpu_simulator.memory")
local RegisterFile  = require("coding_adventures.cpu_simulator.register_file")

return {
    Memory        = memory_mod.Memory,
    SparseMemory  = memory_mod.SparseMemory,
    RegisterFile  = RegisterFile,
}
