-- Hardening tests for sequential.lua: INVALID_BIT identity, bounded widths,
-- validated state on every path, fresh results.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local lg = require("coding_adventures.logic_gates")

local function code_of(f, ...)
    local ok, err = pcall(f, ...)
    assert.is_false(ok)
    return lg.error_code(err), err
end

local BAD_BITS = { 1.0, 0.0, -0.0, 2, -1, "1", true, false, {}, 0/0, math.huge, print }

describe("sequential bit identity", function()
    it("rejects 1.0 and every non-bit in every latch/flip-flop argument", function()
        for _, bad in ipairs(BAD_BITS) do
            assert.are.equal("INVALID_BIT", code_of(lg.SRLatch, bad, 0, 0, 1))
            assert.are.equal("INVALID_BIT", code_of(lg.SRLatch, 0, bad, 0, 1))
            assert.are.equal("INVALID_BIT", code_of(lg.SRLatch, 0, 0, bad, 1))
            assert.are.equal("INVALID_BIT", code_of(lg.SRLatch, 0, 0, 0, bad))
            assert.are.equal("INVALID_BIT", code_of(lg.DLatch, bad, 1, 0, 1))
            assert.are.equal("INVALID_BIT", code_of(lg.DLatch, 1, bad, 0, 1))
            assert.are.equal("INVALID_BIT", code_of(lg.DFlipFlop, bad, 0, nil))
            assert.are.equal("INVALID_BIT", code_of(lg.DFlipFlop, 0, bad, nil))
            assert.are.equal("INVALID_BIT", code_of(lg.Counter, bad, 0, lg.new_counter_state(2)))
            assert.are.equal("INVALID_BIT", code_of(lg.Counter, 0, bad, lg.new_counter_state(2)))
            assert.are.equal("INVALID_BIT", code_of(lg.Register, { bad }, 0, nil))
            assert.are.equal("INVALID_BIT", code_of(lg.Register, { 1 }, bad, nil))
            assert.are.equal("INVALID_BIT", code_of(lg.ShiftRegister, bad, 0, { lg.new_flip_flop_state() }, "left"))
            assert.are.equal("INVALID_BIT", code_of(lg.ShiftRegister, 0, bad, { lg.new_flip_flop_state() }, "left"))
        end
    end)

    it("is_invalid_bit recognises the structured error", function()
        local _, err = code_of(lg.DLatch, 1.0, 1, 0, 1)
        assert.is_true(lg.is_invalid_bit(err))
        assert.are.equal("data", err.param)
    end)

    it("rejects bad flip-flop state fields (1.0, nil, garbage)", function()
        for _, bad in ipairs(BAD_BITS) do
            local st = lg.new_flip_flop_state()
            st.slave_q = bad
            assert.are.equal("INVALID_BIT", code_of(lg.DFlipFlop, 1, 0, st))
            assert.are.equal("INVALID_BIT", code_of(lg.Register, { 1 }, 0, { st }))
            assert.are.equal("INVALID_BIT", code_of(lg.ShiftRegister, 1, 0, { st }, "right"))
        end
        local st = lg.new_flip_flop_state()
        st.master_q_bar = nil
        assert.are.equal("INVALID_BIT", code_of(lg.DFlipFlop, 1, 0, st))
        assert.are.equal("INVALID_BIT", code_of(lg.new_flip_flop_state, 1.0))
    end)

    it("rejects non-table flip-flop states with INVALID_SHAPE", function()
        assert.are.equal("INVALID_SHAPE", code_of(lg.DFlipFlop, 1, 0, 5))
        assert.are.equal("INVALID_SHAPE", code_of(lg.Register, { 1 }, 0, { 5 }))
        assert.are.equal("INVALID_SHAPE", code_of(lg.Register, "abc", 0, nil))
        assert.are.equal("INVALID_SHAPE", code_of(lg.ShiftRegister, 0, 0, {}, "left"))
        assert.are.equal("INVALID_SHAPE", code_of(lg.ShiftRegister, 0, 0, nil, "left"))
        assert.are.equal("INVALID_SHAPE", code_of(lg.ShiftRegister, 0, 0, { lg.new_flip_flop_state() }, "up"))
        assert.are.equal("INVALID_SHAPE", code_of(lg.Register, {}, 0, nil))
        local two = { lg.new_flip_flop_state(), lg.new_flip_flop_state() }
        assert.are.equal("INVALID_SHAPE", code_of(lg.Register, { 1, 0, 1 }, 1, two))
    end)

    it("Register/ShiftRegister return canonical integers in fresh tables", function()
        local data = { 1, 0, 1 }
        local out, st = lg.Register(data, 0, nil)
        for i = 1, 3 do assert.are.equal("integer", math.type(out[i])) end
        out[1] = 99
        assert.are_not.equal(99, st[1].slave_q)
        local _, so, _ = lg.ShiftRegister(1, 0, { lg.new_flip_flop_state() }, "left")
        assert.are.equal("integer", math.type(so))
    end)

    it("Register rejects oversized data before processing", function()
        local big = {}
        for i = 1, lg.MAX_WIDTH + 1 do big[i] = 0 end
        assert.are.equal("INVALID_WIDTH", code_of(lg.Register, big, 0, nil))
    end)
end)

describe("counter widths", function()
    it("accepts 1 and 64, rejects 0 and 65", function()
        assert.are.equal(64, #lg.new_counter_state(64).bits)
        assert.are.equal(1, #lg.new_counter_state(1).bits)
        assert.are.equal("INVALID_WIDTH", code_of(lg.new_counter_state, 0))
        assert.are.equal("INVALID_WIDTH", code_of(lg.new_counter_state, 65))
        assert.are.equal("INVALID_WIDTH", code_of(lg.new_counter_state, -3))
    end)

    it("rejects huge width promptly without allocating", function()
        local t0 = os.clock()
        assert.are.equal("INVALID_WIDTH", code_of(lg.new_counter_state, 1000000000))
        assert.are.equal("INVALID_WIDTH", code_of(lg.new_counter_state, math.maxinteger))
        assert.is_true(os.clock() - t0 < 0.5)
    end)

    it("rejects non-integer widths", function()
        for _, w in ipairs({ 4.0, 4.5, "4", true, {}, 0/0, math.huge, -math.huge }) do
            assert.are.equal("INVALID_WIDTH", code_of(lg.new_counter_state, w))
        end
        assert.are.equal("INVALID_WIDTH", code_of(lg.new_counter_state, nil))
        assert.is_true(lg.is_invalid_width(select(2, code_of(lg.new_counter_state, "x"))))
    end)

    it("a 64-bit counter wraps from all ones", function()
        local st = lg.new_counter_state(64)
        for i = 1, 64 do st.bits[i] = 1 end
        local out
        out, st = lg.Counter(1, 0, st)
        for i = 1, 64 do assert.are.equal(0, out[i]) end
    end)
end)

describe("counter state validation", function()
    local function paths(st)
        return {
            function() return lg.Counter(1, 0, st) end,
            function() return lg.Counter(0, 0, st) end,
            function() return lg.Counter(0, 1, st) end,
            function() return lg.Counter(1, 1, st) end,
        }
    end

    it("rejects hostile state.width on all paths without looping", function()
        for _, w in ipairs({ 1e9, 65, 0, -1, 2.0, 2.5, "2", true, 0/0, math.maxinteger }) do
            local st = { bits = { 0, 0 }, width = w }
            for _, f in ipairs(paths(st)) do
                local ok, err = pcall(f)
                assert.is_false(ok)
                assert.are.equal("INVALID_WIDTH", lg.error_code(err))
            end
        end
        local st = { bits = { 0, 0 } }
        assert.are.equal("INVALID_WIDTH", code_of(lg.Counter, 1, 0, st))
    end)

    it("rejects width / #bits mismatch on all paths", function()
        for _, st in ipairs({
            { bits = { 0, 0, 0 }, width = 2 },
            { bits = { 0 }, width = 2 },
            { bits = {}, width = 2 },
        }) do
            for _, f in ipairs(paths(st)) do
                local ok, err = pcall(f)
                assert.is_false(ok)
                assert.are.equal("INVALID_SHAPE", lg.error_code(err))
            end
        end
        assert.are.equal("INVALID_SHAPE", code_of(lg.Counter, 1, 0, { width = 2 }))
        assert.are.equal("INVALID_SHAPE", code_of(lg.Counter, 1, 0, 7))
        assert.are.equal("INVALID_SHAPE", code_of(lg.Counter, 1, 0, nil))
    end)

    it("rejects bad stored bits on hold, reset and increment", function()
        for _, bad in ipairs(BAD_BITS) do
            local st = { bits = { 0, bad }, width = 2 }
            for _, f in ipairs(paths(st)) do
                local ok, err = pcall(f)
                assert.is_false(ok)
                assert.are.equal("INVALID_BIT", lg.error_code(err))
            end
        end
    end)

    it("returns fresh tables, never the caller's or aliased", function()
        local st = lg.new_counter_state(3)
        local out, st2 = lg.Counter(0, 0, st)
        assert.are_not.equal(st.bits, out)
        assert.are_not.equal(st.bits, st2.bits)
        assert.are_not.equal(out, st2.bits)
        out[1] = 7
        assert.are.equal(0, st2.bits[1])
        local out2, st3 = lg.Counter(0, 1, st)
        assert.are_not.equal(out2, st3.bits)
    end)

    it("still counts correctly", function()
        local st = lg.new_counter_state(3)
        for _i = 1, 5 do local _o; _o, st = lg.Counter(1, 0, st) end
        assert.are.same({ 1, 0, 1 }, st.bits)
    end)
end)
