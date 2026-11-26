-- sequential.lua — public entry point for the sequential circuits.
--
-- The implementation lives in sequential_impl.lua, a factory that gates.lua
-- instantiates once with its private kit.  This module simply returns that
-- (read-only) instance so `require("coding_adventures.logic_gates.sequential")`
-- keeps working while no public path ever yields the kit.
return require("coding_adventures.logic_gates.gates").sequential
