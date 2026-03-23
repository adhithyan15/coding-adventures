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
-- This is Layer 10 of the computing stack. It has no dependencies.

local gates = require("coding_adventures.logic_gates.gates")
local sequential = require("coding_adventures.logic_gates.sequential")

return {
    VERSION = "0.1.0",

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
