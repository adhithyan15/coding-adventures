-- Tests for coding_adventures.clr_simulator
-- ==========================================
--
-- Tests every opcode, edge case, and error condition.
-- Target: 95%+ line coverage.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local clr = require("coding_adventures.clr_simulator")

-- ============================================================================
-- Helpers
-- ============================================================================

--- Assemble and run a program, returning final simulator state and traces.
local function run_program(parts, opts)
    local bytecode = clr.assemble(parts)
    local sim = clr.new()
    sim = clr.load(sim, bytecode, opts)
    local final, traces = clr.run(sim)
    return final, traces
end

--- Run a program and return the top of stack.
local function top_of_stack(parts, opts)
    local sim = run_program(parts, opts)
    return sim.stack[#sim.stack]
end

--- Run a program and return local variable at slot (0-based).
local function local_var(parts, slot, opts)
    local sim = run_program(parts, opts)
    return sim.locals[slot + 1]
end

-- ============================================================================
-- Module basics
-- ============================================================================

describe("clr_simulator module", function()
    it("has a version", function()
        assert.are.equal("0.1.0", clr.VERSION)
    end)

    it("exports opcode constants", function()
        assert.are.equal(0x00, clr.NOP)
        assert.are.equal(0x01, clr.LDNULL)
        assert.are.equal(0x2A, clr.RET)
        assert.are.equal(0x58, clr.ADD)
        assert.are.equal(0x59, clr.SUB)
        assert.are.equal(0x5A, clr.MUL)
        assert.are.equal(0x5B, clr.DIV)
        assert.are.equal(0xFE, clr.PREFIX_FE)
        assert.are.equal(0x01, clr.CEQ_BYTE)
        assert.are.equal(0x02, clr.CGT_BYTE)
        assert.are.equal(0x04, clr.CLT_BYTE)
    end)
end)

-- ============================================================================
-- new() and load()
-- ============================================================================

describe("new and load", function()
    it("new() returns empty simulator", function()
        local sim = clr.new()
        assert.are.equal(0, #sim.stack)
        assert.are.equal(0, sim.pc)
        assert.is_false(sim.halted)
    end)

    it("load() resets state", function()
        local sim = clr.new()
        local bytecode = {clr.RET}
        sim = clr.load(sim, bytecode)
        assert.are.equal(0, sim.pc)
        assert.is_false(sim.halted)
        assert.are.equal(0, #sim.stack)
    end)

    it("load() initializes locals with specified count", function()
        local sim = clr.new()
        sim = clr.load(sim, {clr.RET}, {num_locals = 8})
        assert.are.equal(8, #sim.locals)
        for i = 1, 8 do
            assert.is_nil(sim.locals[i])
        end
    end)

    it("load() defaults to 16 locals", function()
        local sim = clr.new()
        sim = clr.load(sim, {clr.RET})
        assert.are.equal(16, #sim.locals)
    end)
end)

-- ============================================================================
-- NOP
-- ============================================================================

describe("nop", function()
    it("does nothing", function()
        local bytecode = {clr.NOP, clr.RET}
        local sim = clr.new()
        sim = clr.load(sim, bytecode)
        sim, trace = clr.step(sim)
        assert.are.equal("nop", trace.opcode)
        assert.are.equal(1, sim.pc)
        assert.are.equal(0, #sim.stack)
    end)
end)

-- ============================================================================
-- LDNULL
-- ============================================================================

describe("ldnull", function()
    it("pushes nil", function()
        local bytecode = {clr.LDNULL, clr.RET}
        local sim = clr.new()
        sim = clr.load(sim, bytecode)
        sim, trace = clr.step(sim)
        assert.are.equal("ldnull", trace.opcode)
        assert.are.equal(1, #sim.stack)
        assert.is_nil(sim.stack[1])
    end)
end)

-- ============================================================================
-- ldc.i4 variants
-- ============================================================================

describe("ldc.i4 short forms (0-8)", function()
    for n = 0, 8 do
        it(string.format("ldc.i4.%d pushes %d", n, n), function()
            local bytecode = {clr.LDC_I4_0 + n, clr.RET}
            local sim = clr.new()
            sim = clr.load(sim, bytecode)
            sim, _ = clr.step(sim)
            assert.are.equal(n, sim.stack[1])
        end)
    end
end)

describe("ldc.i4.s", function()
    it("pushes a positive value", function()
        local sim, _ = run_program({ {clr.LDC_I4_S, 42}, {clr.RET} })
        assert.are.equal(42, sim.stack[1])
    end)

    it("pushes a negative value", function()
        -- -10 in two's complement signed byte = 246
        local sim, _ = run_program({ {clr.LDC_I4_S, 246}, {clr.RET} })
        assert.are.equal(-10, sim.stack[1])
    end)

    it("handles -1 (0xFF = 255)", function()
        local sim, _ = run_program({ {clr.LDC_I4_S, 255}, {clr.RET} })
        assert.are.equal(-1, sim.stack[1])
    end)
end)

describe("ldc.i4 (32-bit)", function()
    it("pushes a large positive value", function()
        local bytes = clr.encode_ldc_i4(100000)
        bytes[#bytes + 1] = clr.RET
        local sim, _ = run_program({ bytes })
        assert.are.equal(100000, sim.stack[1])
    end)

    it("pushes a large negative value", function()
        local bytes = clr.encode_ldc_i4(-100000)
        bytes[#bytes + 1] = clr.RET
        local sim, _ = run_program({ bytes })
        assert.are.equal(-100000, sim.stack[1])
    end)
end)

-- ============================================================================
-- encode_ldc_i4
-- ============================================================================

describe("encode_ldc_i4", function()
    it("0 → ldc.i4.0 (single byte)", function()
        local bytes = clr.encode_ldc_i4(0)
        assert.are.equal(1, #bytes)
        assert.are.equal(clr.LDC_I4_0, bytes[1])
    end)

    it("8 → ldc.i4.8 (single byte)", function()
        local bytes = clr.encode_ldc_i4(8)
        assert.are.equal(1, #bytes)
        assert.are.equal(clr.LDC_I4_8, bytes[1])
    end)

    it("9 → ldc.i4.s (2 bytes)", function()
        local bytes = clr.encode_ldc_i4(9)
        assert.are.equal(2, #bytes)
        assert.are.equal(clr.LDC_I4_S, bytes[1])
    end)

    it("-1 → ldc.i4.s (2 bytes)", function()
        local bytes = clr.encode_ldc_i4(-1)
        assert.are.equal(2, #bytes)
        assert.are.equal(clr.LDC_I4_S, bytes[1])
    end)

    it("large → ldc.i4 (5 bytes)", function()
        local bytes = clr.encode_ldc_i4(50000)
        assert.are.equal(5, #bytes)
        assert.are.equal(clr.LDC_I4, bytes[1])
    end)
end)

-- ============================================================================
-- stloc / ldloc
-- ============================================================================

describe("stloc and ldloc short forms (0-3)", function()
    for slot = 0, 3 do
        it(string.format("stloc.%d and ldloc.%d roundtrip", slot, slot), function()
            -- Push 10+slot, store, load, ret
            local code = clr.assemble({
                clr.encode_ldc_i4(10 + slot),
                { clr.STLOC_0 + slot },
                { clr.LDLOC_0 + slot },
                { clr.RET },
            })
            local sim = clr.new()
            sim = clr.load(sim, code)
            local final, _ = clr.run(sim)
            assert.are.equal(10 + slot, final.stack[1])
            assert.are.equal(10 + slot, final.locals[slot + 1])
        end)
    end
end)

describe("stloc.s and ldloc.s", function()
    it("store and load slot 5", function()
        local code = clr.assemble({
            clr.encode_ldc_i4(99),
            { clr.STLOC_S, 5 },
            { clr.LDLOC_S, 5 },
            { clr.RET },
        })
        local sim = clr.new()
        sim = clr.load(sim, code, {num_locals = 16})
        local final, _ = clr.run(sim)
        assert.are.equal(99, final.stack[1])
        assert.are.equal(99, final.locals[6])  -- slot 5 = index 6 (1-based)
    end)
end)

-- ============================================================================
-- Arithmetic
-- ============================================================================

describe("add", function()
    it("1 + 2 = 3", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(1),
            clr.encode_ldc_i4(2),
            { clr.ADD },
            { clr.RET },
        })
        assert.are.equal(3, sim.stack[1])
    end)

    it("handles negative operands", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(-5),
            clr.encode_ldc_i4(3),
            { clr.ADD },
            { clr.RET },
        })
        assert.are.equal(-2, sim.stack[1])
    end)
end)

describe("sub", function()
    it("10 - 3 = 7", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(10),
            clr.encode_ldc_i4(3),
            { clr.SUB },
            { clr.RET },
        })
        assert.are.equal(7, sim.stack[1])
    end)
end)

describe("mul", function()
    it("4 * 5 = 20", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(4),
            clr.encode_ldc_i4(5),
            { clr.MUL },
            { clr.RET },
        })
        assert.are.equal(20, sim.stack[1])
    end)
end)

describe("div", function()
    it("10 / 2 = 5", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(10),
            clr.encode_ldc_i4(2),
            { clr.DIV },
            { clr.RET },
        })
        assert.are.equal(5, sim.stack[1])
    end)

    it("truncates toward zero: 7 / 2 = 3", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(7),
            clr.encode_ldc_i4(2),
            { clr.DIV },
            { clr.RET },
        })
        assert.are.equal(3, sim.stack[1])
    end)

    it("raises on division by zero", function()
        local bytecode = clr.assemble({
            clr.encode_ldc_i4(5),
            clr.encode_ldc_i4(0),
            { clr.DIV },
            { clr.RET },
        })
        local sim = clr.new()
        sim = clr.load(sim, bytecode)
        sim, _ = clr.step(sim)  -- push 5
        sim, _ = clr.step(sim)  -- push 0
        assert.has_error(function() clr.step(sim) end)
    end)
end)

-- ============================================================================
-- Compare instructions (two-byte opcodes)
-- ============================================================================

describe("ceq (0xFE 0x01)", function()
    it("equal values → push 1", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(5),
            clr.encode_ldc_i4(5),
            { clr.PREFIX_FE, clr.CEQ_BYTE },
            { clr.RET },
        })
        assert.are.equal(1, sim.stack[1])
    end)

    it("unequal values → push 0", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(3),
            clr.encode_ldc_i4(5),
            { clr.PREFIX_FE, clr.CEQ_BYTE },
            { clr.RET },
        })
        assert.are.equal(0, sim.stack[1])
    end)
end)

describe("cgt (0xFE 0x02)", function()
    it("a > b → push 1", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(10),
            clr.encode_ldc_i4(3),
            { clr.PREFIX_FE, clr.CGT_BYTE },
            { clr.RET },
        })
        assert.are.equal(1, sim.stack[1])
    end)

    it("a <= b → push 0", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(3),
            clr.encode_ldc_i4(10),
            { clr.PREFIX_FE, clr.CGT_BYTE },
            { clr.RET },
        })
        assert.are.equal(0, sim.stack[1])
    end)
end)

describe("clt (0xFE 0x04)", function()
    it("a < b → push 1", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(2),
            clr.encode_ldc_i4(8),
            { clr.PREFIX_FE, clr.CLT_BYTE },
            { clr.RET },
        })
        assert.are.equal(1, sim.stack[1])
    end)

    it("a >= b → push 0", function()
        local sim, _ = run_program({
            clr.encode_ldc_i4(8),
            clr.encode_ldc_i4(2),
            { clr.PREFIX_FE, clr.CLT_BYTE },
            { clr.RET },
        })
        assert.are.equal(0, sim.stack[1])
    end)
end)

-- ============================================================================
-- Branch instructions
-- ============================================================================

describe("br.s (unconditional branch)", function()
    -- Program: push 1, br.s (skip next push), push 2 (skipped), push 3, ret
    -- Result: stack = [1, 3]
    it("skips over instructions", function()
        local code = clr.assemble({
            clr.encode_ldc_i4(1),  -- pc 0: push 1
            { clr.BR_S, 1 },       -- pc 1: branch offset=1 → skip next 1 byte
            clr.encode_ldc_i4(2),  -- pc 3: (skipped)
            clr.encode_ldc_i4(3),  -- pc 4: push 3
            { clr.RET },           -- pc 5
        })
        local sim = clr.new()
        sim = clr.load(sim, code)
        local final, _ = clr.run(sim)
        assert.are.equal(1, final.stack[1])
        assert.are.equal(3, final.stack[2])
    end)

    it("negative offset creates backward jump", function()
        -- Counter: push 0, store loc0.
        -- Loop: load loc0, push 1, add, store loc0, load loc0, push 3, clt, brfalse → exit
        -- This adds 1 to x until x >= 3.
        -- We build manually for precision:
        -- 0: ldc.i4.0    (1 byte)  → [0x16]
        -- 1: stloc.0     (1 byte)  → [0x0A]
        -- 2: ldloc.0     (1 byte)  → [0x06]
        -- 3: ldc.i4.1    (1 byte)  → [0x17]
        -- 4: add         (1 byte)  → [0x58]
        -- 5: stloc.0     (1 byte)  → [0x0A]
        -- 6: ldloc.0     (1 byte)  → [0x06]
        -- 7: ldc.i4.3    (1 byte)  → [0x19]
        -- 8: clt         (2 bytes) → [0xFE, 0x04]
        -- 10: brtrue.s -10 (2 bytes) → [0x2D, 0xF6] (offset=-10, 0xF6=246)
        -- 12: ldloc.0    (1 byte)  → [0x06]
        -- 13: ret        (1 byte)  → [0x2A]
        -- brtrue.s offset: target=2, next_pc=12, offset=2-12=-10, signed byte=-10 → 246
        local code = {
            0x16,        -- ldc.i4.0
            0x0A,        -- stloc.0
            0x06,        -- ldloc.0  (loop start at pc=2)
            0x17,        -- ldc.i4.1
            0x58,        -- add
            0x0A,        -- stloc.0
            0x06,        -- ldloc.0
            0x19,        -- ldc.i4.3
            0xFE, 0x04,  -- clt
            0x2D, 246,   -- brtrue.s -10 (246 = -10 as signed byte)
            0x06,        -- ldloc.0
            0x2A,        -- ret
        }
        local sim = clr.new()
        sim = clr.load(sim, code)
        local final, _ = clr.run(sim)
        assert.are.equal(3, final.stack[1])
    end)
end)

describe("brfalse.s", function()
    it("branches when value is 0", function()
        -- push 0, brfalse.s +1 (skip push 42), push 99, ret → stack=[99]
        local code = clr.assemble({
            clr.encode_ldc_i4(0),
            { clr.BRFALSE_S, 1 },   -- skip next 1-byte instruction
            clr.encode_ldc_i4(42),  -- skipped
            clr.encode_ldc_i4(99),
            { clr.RET },
        })
        local sim = clr.new()
        sim = clr.load(sim, code)
        local final, _ = clr.run(sim)
        assert.are.equal(99, final.stack[1])
        assert.is_nil(final.stack[2])
    end)

    it("does not branch when value is non-zero", function()
        -- push 1, brfalse.s +1 (not taken), push 42, push 99, ret → stack=[42,99]
        local code = clr.assemble({
            clr.encode_ldc_i4(1),
            { clr.BRFALSE_S, 1 },
            clr.encode_ldc_i4(42),
            clr.encode_ldc_i4(99),
            { clr.RET },
        })
        local sim = clr.new()
        sim = clr.load(sim, code)
        local final, _ = clr.run(sim)
        assert.are.equal(42, final.stack[1])
        assert.are.equal(99, final.stack[2])
    end)

    it("treats null as false (branches)", function()
        local code = {
            clr.LDNULL,       -- push nil
            clr.BRFALSE_S, 1, -- branch over next byte
            clr.LDC_I4_1,    -- skipped
            clr.LDC_I4_2,    -- push 2
            clr.RET,
        }
        local sim = clr.new()
        sim = clr.load(sim, code)
        local final, _ = clr.run(sim)
        assert.are.equal(2, final.stack[1])
    end)
end)

describe("brtrue.s", function()
    it("branches when value is non-zero", function()
        -- push 1, brtrue.s +1 (skip push 42), push 99, ret → stack=[99]
        local code = clr.assemble({
            clr.encode_ldc_i4(1),
            { clr.BRTRUE_S, 1 },
            clr.encode_ldc_i4(42),
            clr.encode_ldc_i4(99),
            { clr.RET },
        })
        local sim = clr.new()
        sim = clr.load(sim, code)
        local final, _ = clr.run(sim)
        assert.are.equal(99, final.stack[1])
    end)

    it("does not branch when value is 0", function()
        local code = clr.assemble({
            clr.encode_ldc_i4(0),
            { clr.BRTRUE_S, 1 },
            clr.encode_ldc_i4(42),
            clr.encode_ldc_i4(99),
            { clr.RET },
        })
        local sim = clr.new()
        sim = clr.load(sim, code)
        local final, _ = clr.run(sim)
        assert.are.equal(42, final.stack[1])
        assert.are.equal(99, final.stack[2])
    end)
end)

-- ============================================================================
-- ret
-- ============================================================================

describe("ret", function()
    it("halts the simulator", function()
        local sim = clr.new()
        sim = clr.load(sim, {clr.RET})
        sim, _ = clr.step(sim)
        assert.is_true(sim.halted)
    end)

    it("step after halt raises error", function()
        local sim = clr.new()
        sim = clr.load(sim, {clr.RET})
        sim, _ = clr.step(sim)
        assert.has_error(function() clr.step(sim) end)
    end)
end)

-- ============================================================================
-- run()
-- ============================================================================

describe("run", function()
    it("returns all traces", function()
        -- 3 instructions: ldc.i4.5, ldc.i4.3, add, ret → 4 traces
        local code = clr.assemble({
            clr.encode_ldc_i4(5),
            clr.encode_ldc_i4(3),
            { clr.ADD },
            { clr.RET },
        })
        local sim = clr.new()
        sim = clr.load(sim, code)
        local final, traces = clr.run(sim)
        assert.are.equal(4, #traces)
        assert.are.equal(8, final.stack[1])
    end)

    it("stops at max_steps", function()
        -- Infinite loop: br.s -2 (loops forever)
        local code = { clr.BR_S, 254 }  -- offset=-2, 254=256-2
        local sim = clr.new()
        sim = clr.load(sim, code)
        local final, traces = clr.run(sim, {max_steps = 5})
        assert.are.equal(5, #traces)
        assert.is_false(final.halted)
    end)
end)

-- ============================================================================
-- Trace contents
-- ============================================================================

describe("trace records", function()
    it("captures pc, opcode, stack_before, stack_after, description", function()
        local code = {clr.LDC_I4_3, clr.RET}
        local sim = clr.new()
        sim = clr.load(sim, code)
        sim, trace = clr.step(sim)
        assert.are.equal(0, trace.pc)
        assert.are.equal("ldc.i4.3", trace.opcode)
        assert.are.equal(0, #trace.stack_before)
        assert.are.equal(1, #trace.stack_after)
        assert.are.equal(3, trace.stack_after[1])
        assert.is_not_nil(trace.description)
    end)
end)

-- ============================================================================
-- Error conditions
-- ============================================================================

describe("error conditions", function()
    it("raises on stack underflow (add with empty stack)", function()
        local sim = clr.new()
        sim = clr.load(sim, {clr.ADD, clr.RET})
        assert.has_error(function() clr.step(sim) end)
    end)

    it("raises on PC past end of bytecode", function()
        local sim = clr.new()
        sim = clr.load(sim, {})
        assert.has_error(function() clr.step(sim) end)
    end)

    it("raises on unknown opcode", function()
        local sim = clr.new()
        sim = clr.load(sim, {0xFF})  -- not a valid CLR opcode
        assert.has_error(function() clr.step(sim) end)
    end)

    it("raises on incomplete two-byte opcode", function()
        local sim = clr.new()
        sim = clr.load(sim, {clr.PREFIX_FE})  -- missing second byte
        sim = clr.load(sim, {clr.LDC_I4_1, clr.LDC_I4_1, clr.PREFIX_FE})
        sim, _ = clr.step(sim)
        sim, _ = clr.step(sim)
        assert.has_error(function() clr.step(sim) end)
    end)

    it("raises on unknown 0xFE subopcode", function()
        local sim = clr.new()
        sim = clr.load(sim, {
            clr.LDC_I4_1,
            clr.LDC_I4_1,
            clr.PREFIX_FE, 0xFF  -- 0xFF is not ceq/cgt/clt
        })
        sim, _ = clr.step(sim)
        sim, _ = clr.step(sim)
        assert.has_error(function() clr.step(sim) end)
    end)

    it("raises on uninitialized local variable (ldloc.0)", function()
        local sim = clr.new()
        sim = clr.load(sim, {clr.LDLOC_0, clr.RET})
        assert.has_error(function() clr.step(sim) end)
    end)

    it("raises on uninitialized local variable (ldloc.s)", function()
        local sim = clr.new()
        sim = clr.load(sim, {clr.LDLOC_S, 5, clr.RET})
        assert.has_error(function() clr.step(sim) end)
    end)
end)

-- ============================================================================
-- Integration test: x = 1 + 2 (the canonical CLR example)
-- ============================================================================

describe("integration: x = 1 + 2", function()
    it("produces x = 3 in local[0]", function()
        -- ldc.i4.1 / ldc.i4.2 / add / stloc.0 / ret
        local code = clr.assemble({
            { clr.LDC_I4_1 },  -- 0x17
            { clr.LDC_I4_2 },  -- 0x18
            { clr.ADD },       -- 0x58
            { clr.STLOC_0 },   -- 0x0A
            { clr.RET },       -- 0x2A
        })
        local sim = clr.new()
        sim = clr.load(sim, code)
        local final, traces = clr.run(sim)
        assert.is_true(final.halted)
        assert.are.equal(3, final.locals[1])
        assert.are.equal(5, #traces)
    end)
end)

-- ============================================================================
-- Integration test: max(a, b)
-- ============================================================================

describe("integration: max(5, 8) = 8", function()
    -- Implements: result = a > b ? a : b
    -- Using: cgt + brfalse.s to select the larger value
    it("returns the larger value", function()
        -- locals[0] = a = 5, locals[1] = b = 8
        -- 0: ldc.i4.5   1 byte
        -- 1: stloc.0    1 byte
        -- 2: ldc.i4.8   1 byte (0x1E)
        -- 3: stloc.1    1 byte
        -- 4: ldloc.0    1 byte  ← load a
        -- 5: ldloc.1    1 byte  ← load b
        -- 6: FE 02      2 bytes ← cgt (a > b?)
        -- 8: brfalse.s  +2 (2 bytes) → if false, jump to 12 (8+2+2=12)
        -- 10: ldloc.0   1 byte  ← a (the max)
        -- 11: br.s      +1 (2 bytes) → jump to 13 (11+2+1=14)
        -- 13: ldloc.1   1 byte  ← b (the max)
        -- 14: ret       1 byte
        local code = {
            0x1B,           -- ldc.i4.5
            0x0A,           -- stloc.0
            0x1E,           -- ldc.i4.8
            0x0B,           -- stloc.1
            0x06,           -- ldloc.0 (a)
            0x07,           -- ldloc.1 (b)
            0xFE, 0x02,     -- cgt
            0x2C, 2,        -- brfalse.s +2 → jump to load-b if NOT a>b
            0x06,           -- ldloc.0 (return a)
            0x2B, 1,        -- br.s +1 → jump over load-b
            0x07,           -- ldloc.1 (return b)
            0x2A,           -- ret
        }
        local sim = clr.new()
        sim = clr.load(sim, code)
        local final, _ = clr.run(sim)
        assert.are.equal(8, final.stack[1])
    end)
end)

-- ============================================================================
-- assemble helper
-- ============================================================================

describe("assemble", function()
    it("flattens nested byte arrays", function()
        local result = clr.assemble({
            {0x10, 0x20},
            {0x30},
            {0x40, 0x50},
        })
        assert.are.same({0x10, 0x20, 0x30, 0x40, 0x50}, result)
    end)
end)
