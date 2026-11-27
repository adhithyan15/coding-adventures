-- logic-gates — The fundamental building blocks of all digital circuits
--
-- Every computation your CPU performs — from adding numbers to rendering
-- 3D graphics — ultimately reduces to billions of tiny switches called
-- transistors flipping between on (1) and off (0). Logic gates are the
-- first abstraction layer above transistors: they combine a few transistors
-- into a circuit that performs a Boolean operation.
--
-- From just NAND gates (or just NOR gates), you can build every other gate.
-- From gates, you build adders. From adders, you build ALUs. From ALUs,
-- you build CPUs. This package implements that foundational layer.
--
-- This is Layer 10 of the computing stack.

local gates = require("coding_adventures.logic_gates.gates")
local sequential = require("coding_adventures.logic_gates.sequential")

-- The public table is a read-only proxy: writing an export raises, and the
-- metatable is locked.  (`rawset` / the debug library can still defeat it;
-- that is the host-sandbox boundary.  Internal calls never go through this
-- table, so even a successful rawset cannot change gate behaviour.)
local setmetatable, next, error = setmetatable, next, error

local api = {
    VERSION = "0.6.0",

    -- Combinational gates
    AND = gates.AND,
    OR = gates.OR,
    NOT = gates.NOT,
    XOR = gates.XOR,
    NAND = gates.NAND,
    NOR = gates.NOR,
    XNOR = gates.XNOR,

    -- NAND-derived (proving functional completeness)
    NAND_NOT = gates.NAND_NOT,
    NAND_AND = gates.NAND_AND,
    NAND_OR = gates.NAND_OR,
    NAND_XOR = gates.NAND_XOR,

    -- Error identity (stable across lanes: err.code == "INVALID_BIT")
    is_invalid_bit = gates.is_invalid_bit,
    is_invalid_arity = gates.is_invalid_arity,
    is_invalid_width = gates.is_invalid_width,
    is_invalid_shape = gates.is_invalid_shape,
    is_internal = gates.is_internal,
    MAX_INPUTS = gates.MAX_INPUTS,
    error_code = gates.error_code,
    MAX_WIDTH = sequential.MAX_WIDTH,
    MAX_COUNTER_WIDTH = sequential.MAX_COUNTER_WIDTH,

    -- Multi-input
    ANDn = gates.ANDn,
    ORn = gates.ORn,

    -- Sequential logic
    SRLatch = sequential.SRLatch,
    DLatch = sequential.DLatch,
    DFlipFlop = sequential.DFlipFlop,
    Register = sequential.Register,
    ShiftRegister = sequential.ShiftRegister,
    Counter = sequential.Counter,

    -- State constructors
    new_flip_flop_state = sequential.new_flip_flop_state,
    new_counter_state = sequential.new_counter_state,
}

return setmetatable({}, {
    __index = api,
    __newindex = function() error("logic_gates: module is read-only", 2) end,
    __metatable = "logic_gates module (read-only)",
    __pairs = function() return next, api, nil end,
})
