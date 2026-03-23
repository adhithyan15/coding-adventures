-- Tests for logic-gates — comprehensive truth table verification
--
-- Logic gates are fully specified by their truth tables. Every gate gets
-- tested against its complete truth table, plus edge cases for invalid
-- inputs and multi-input variants.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local lg = require("coding_adventures.logic_gates")

-- =========================================================================
-- Combinational Gates — Truth Table Tests
-- =========================================================================

describe("AND gate", function()
    it("returns 1 only when both inputs are 1", function()
        assert.are.equal(0, lg.AND(0, 0))
        assert.are.equal(0, lg.AND(0, 1))
        assert.are.equal(0, lg.AND(1, 0))
        assert.are.equal(1, lg.AND(1, 1))
    end)

    it("errors on invalid inputs", function()
        assert.has_error(function() lg.AND(2, 0) end)
        assert.has_error(function() lg.AND(0, -1) end)
    end)
end)

describe("OR gate", function()
    it("returns 1 when at least one input is 1", function()
        assert.are.equal(0, lg.OR(0, 0))
        assert.are.equal(1, lg.OR(0, 1))
        assert.are.equal(1, lg.OR(1, 0))
        assert.are.equal(1, lg.OR(1, 1))
    end)

    it("errors on invalid inputs", function()
        assert.has_error(function() lg.OR(2, 0) end)
    end)
end)

describe("NOT gate", function()
    it("inverts its input", function()
        assert.are.equal(1, lg.NOT(0))
        assert.are.equal(0, lg.NOT(1))
    end)

    it("errors on invalid inputs", function()
        assert.has_error(function() lg.NOT(2) end)
        assert.has_error(function() lg.NOT(-1) end)
    end)
end)

describe("XOR gate", function()
    it("returns 1 when inputs differ", function()
        assert.are.equal(0, lg.XOR(0, 0))
        assert.are.equal(1, lg.XOR(0, 1))
        assert.are.equal(1, lg.XOR(1, 0))
        assert.are.equal(0, lg.XOR(1, 1))
    end)

    it("errors on invalid inputs", function()
        assert.has_error(function() lg.XOR(0, 3) end)
    end)
end)

describe("NAND gate", function()
    it("returns 0 only when both inputs are 1", function()
        assert.are.equal(1, lg.NAND(0, 0))
        assert.are.equal(1, lg.NAND(0, 1))
        assert.are.equal(1, lg.NAND(1, 0))
        assert.are.equal(0, lg.NAND(1, 1))
    end)
end)

describe("NOR gate", function()
    it("returns 1 only when both inputs are 0", function()
        assert.are.equal(1, lg.NOR(0, 0))
        assert.are.equal(0, lg.NOR(0, 1))
        assert.are.equal(0, lg.NOR(1, 0))
        assert.are.equal(0, lg.NOR(1, 1))
    end)
end)

describe("XNOR gate", function()
    it("returns 1 when inputs are the same", function()
        assert.are.equal(1, lg.XNOR(0, 0))
        assert.are.equal(0, lg.XNOR(0, 1))
        assert.are.equal(0, lg.XNOR(1, 0))
        assert.are.equal(1, lg.XNOR(1, 1))
    end)
end)

-- =========================================================================
-- NAND-Derived Gates — Functional Completeness
-- =========================================================================

describe("NAND-derived gates", function()
    it("NAND_NOT matches NOT for all inputs", function()
        assert.are.equal(lg.NOT(0), lg.NAND_NOT(0))
        assert.are.equal(lg.NOT(1), lg.NAND_NOT(1))
    end)

    it("NAND_AND matches AND for all input combinations", function()
        for _, a in ipairs({0, 1}) do
            for _, b in ipairs({0, 1}) do
                assert.are.equal(lg.AND(a, b), lg.NAND_AND(a, b),
                    string.format("NAND_AND(%d,%d)", a, b))
            end
        end
    end)

    it("NAND_OR matches OR for all input combinations", function()
        for _, a in ipairs({0, 1}) do
            for _, b in ipairs({0, 1}) do
                assert.are.equal(lg.OR(a, b), lg.NAND_OR(a, b),
                    string.format("NAND_OR(%d,%d)", a, b))
            end
        end
    end)

    it("NAND_XOR matches XOR for all input combinations", function()
        for _, a in ipairs({0, 1}) do
            for _, b in ipairs({0, 1}) do
                assert.are.equal(lg.XOR(a, b), lg.NAND_XOR(a, b),
                    string.format("NAND_XOR(%d,%d)", a, b))
            end
        end
    end)
end)

-- =========================================================================
-- Multi-Input Gates
-- =========================================================================

describe("ANDn", function()
    it("returns 1 only when all inputs are 1", function()
        assert.are.equal(1, lg.ANDn(1, 1))
        assert.are.equal(0, lg.ANDn(1, 0))
        assert.are.equal(0, lg.ANDn(0, 1))
        assert.are.equal(1, lg.ANDn(1, 1, 1))
        assert.are.equal(0, lg.ANDn(1, 1, 0))
        assert.are.equal(1, lg.ANDn(1, 1, 1, 1))
        assert.are.equal(0, lg.ANDn(1, 1, 1, 0))
    end)

    it("errors with fewer than 2 inputs", function()
        assert.has_error(function() lg.ANDn(1) end)
        assert.has_error(function() lg.ANDn() end)
    end)

    it("errors on invalid inputs", function()
        assert.has_error(function() lg.ANDn(1, 2) end)
    end)
end)

describe("ORn", function()
    it("returns 1 when at least one input is 1", function()
        assert.are.equal(0, lg.ORn(0, 0))
        assert.are.equal(1, lg.ORn(0, 1))
        assert.are.equal(1, lg.ORn(1, 0))
        assert.are.equal(0, lg.ORn(0, 0, 0))
        assert.are.equal(1, lg.ORn(0, 0, 1))
        assert.are.equal(0, lg.ORn(0, 0, 0, 0))
        assert.are.equal(1, lg.ORn(0, 0, 0, 1))
    end)

    it("errors with fewer than 2 inputs", function()
        assert.has_error(function() lg.ORn(0) end)
    end)
end)

-- =========================================================================
-- Sequential Logic
-- =========================================================================

describe("SRLatch", function()
    it("sets Q to 1 when Set=1, Reset=0", function()
        local q, q_bar = lg.SRLatch(1, 0, 0, 1)
        assert.are.equal(1, q)
        assert.are.equal(0, q_bar)
    end)

    it("resets Q to 0 when Set=0, Reset=1", function()
        local q, q_bar = lg.SRLatch(0, 1, 1, 0)
        assert.are.equal(0, q)
        assert.are.equal(1, q_bar)
    end)

    it("holds state when Set=0, Reset=0", function()
        local q, q_bar = lg.SRLatch(0, 0, 1, 0)
        assert.are.equal(1, q)
        assert.are.equal(0, q_bar)

        q, q_bar = lg.SRLatch(0, 0, 0, 1)
        assert.are.equal(0, q)
        assert.are.equal(1, q_bar)
    end)

    it("handles invalid state S=1, R=1", function()
        local q, q_bar = lg.SRLatch(1, 1, 0, 1)
        assert.are.equal(0, q)
        assert.are.equal(0, q_bar)
    end)
end)

describe("DLatch", function()
    it("follows data when enabled", function()
        local q, q_bar = lg.DLatch(1, 1, 0, 1)
        assert.are.equal(1, q)
        assert.are.equal(0, q_bar)

        q, q_bar = lg.DLatch(0, 1, 1, 0)
        assert.are.equal(0, q)
        assert.are.equal(1, q_bar)
    end)

    it("holds state when disabled", function()
        local q, q_bar = lg.DLatch(1, 0, 0, 1)
        assert.are.equal(0, q)
        assert.are.equal(1, q_bar)
    end)
end)

describe("DFlipFlop", function()
    it("captures data on clock HIGH, outputs on clock LOW", function()
        local state = lg.new_flip_flop_state()
        local _, _, state2 = lg.DFlipFlop(1, 1, state)
        local q, _, _ = lg.DFlipFlop(1, 0, state2)
        assert.are.equal(1, q)
    end)

    it("initializes with nil state", function()
        local q, q_bar, _ = lg.DFlipFlop(0, 0, nil)
        assert.are.equal(0, q)
        assert.are.equal(1, q_bar)
    end)

    it("errors on invalid inputs", function()
        assert.has_error(function() lg.DFlipFlop(2, 0, nil) end)
        assert.has_error(function() lg.DFlipFlop(0, 2, nil) end)
    end)
end)

describe("Register", function()
    it("stores multiple bits", function()
        local data = {1, 0, 1, 1}
        local _, state = lg.Register(data, 1, nil)
        local outputs, _ = lg.Register(data, 0, state)
        assert.are.same({1, 0, 1, 1}, outputs)
    end)

    it("errors on empty data", function()
        assert.has_error(function() lg.Register({}, 1, nil) end)
    end)

    it("errors on data/state length mismatch", function()
        local state = {lg.new_flip_flop_state(), lg.new_flip_flop_state()}
        assert.has_error(function() lg.Register({1, 0, 1}, 1, state) end)
    end)
end)

describe("ShiftRegister", function()
    it("shifts bits left", function()
        local state = {}
        for _ = 1, 4 do
            state[#state + 1] = lg.new_flip_flop_state()
        end

        local _, _, state2 = lg.ShiftRegister(1, 1, state, "left")
        local outputs, serial_out, _ = lg.ShiftRegister(1, 0, state2, "left")
        assert.are.equal(1, outputs[1])
        assert.are.equal(0, serial_out)
    end)

    it("errors on invalid direction", function()
        local state = {lg.new_flip_flop_state()}
        assert.has_error(function() lg.ShiftRegister(0, 1, state, "up") end)
    end)

    it("errors on empty state", function()
        assert.has_error(function() lg.ShiftRegister(0, 1, {}, "left") end)
        assert.has_error(function() lg.ShiftRegister(0, 1, nil, "left") end)
    end)
end)

describe("Counter", function()
    it("counts up on clock pulses", function()
        local state = lg.new_counter_state(4)
        for _ = 1, 3 do
            _, state = lg.Counter(1, 0, state)
        end
        assert.are.same({1, 1, 0, 0}, state.bits)
    end)

    it("wraps around at maximum", function()
        local state = lg.new_counter_state(2)
        for _ = 1, 4 do
            _, state = lg.Counter(1, 0, state)
        end
        assert.are.same({0, 0}, state.bits)
    end)

    it("resets to zero", function()
        local state = lg.new_counter_state(4)
        for _ = 1, 5 do
            _, state = lg.Counter(1, 0, state)
        end
        local outputs
        outputs, state = lg.Counter(0, 1, state)
        assert.are.same({0, 0, 0, 0}, outputs)
    end)

    it("holds on clock=0", function()
        local state = lg.new_counter_state(4)
        _, state = lg.Counter(1, 0, state)
        local outputs
        outputs, _ = lg.Counter(0, 0, state)
        assert.are.same({1, 0, 0, 0}, outputs)
    end)

    it("errors on nil state", function()
        assert.has_error(function() lg.Counter(1, 0, nil) end)
    end)

    it("errors on invalid width", function()
        assert.has_error(function() lg.new_counter_state(0) end)
    end)
end)

-- =========================================================================
-- Version
-- =========================================================================

describe("version", function()
    it("has a version string", function()
        assert.are.equal("0.1.0", lg.VERSION)
    end)
end)
