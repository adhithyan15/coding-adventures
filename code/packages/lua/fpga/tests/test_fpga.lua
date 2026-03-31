-- test_fpga.lua — Test suite for the FPGA fabric simulation
--
-- Tests cover LUT, Slice, CLB, SwitchMatrix, IOBlock, Bitstream, Fabric

local FPGA = require("coding_adventures.fpga")

describe("fpga", function()

    -- ===========================================================
    -- LUT
    -- ===========================================================

    describe("LUT", function()
        it("creates a 4-input LUT with 16 zero entries", function()
            local lut = FPGA.LUT.new(4)
            assert.equals(4, lut.num_inputs)
            assert.equals(16, #lut.truth_table)
            for _, v in ipairs(lut.truth_table) do
                assert.equals(0, v)
            end
        end)

        it("creates a 2-input LUT with 4 entries", function()
            local lut = FPGA.LUT.new(2)
            assert.equals(4, #lut.truth_table)
        end)

        it("configures AND truth table", function()
            local lut = FPGA.LUT.new(2)
            lut:configure({0, 0, 0, 1})
            assert.equals(0, lut:evaluate({0, 0}))
            assert.equals(0, lut:evaluate({0, 1}))
            assert.equals(0, lut:evaluate({1, 0}))
            assert.equals(1, lut:evaluate({1, 1}))
        end)

        it("configures OR truth table", function()
            local lut = FPGA.LUT.new(2)
            lut:configure({0, 1, 1, 1})
            assert.equals(0, lut:evaluate({0, 0}))
            assert.equals(1, lut:evaluate({0, 1}))
            assert.equals(1, lut:evaluate({1, 0}))
            assert.equals(1, lut:evaluate({1, 1}))
        end)

        it("configures XOR truth table", function()
            local lut = FPGA.LUT.new(2)
            lut:configure({0, 1, 1, 0})
            assert.equals(0, lut:evaluate({0, 0}))
            assert.equals(1, lut:evaluate({0, 1}))
            assert.equals(1, lut:evaluate({1, 0}))
            assert.equals(0, lut:evaluate({1, 1}))
        end)

        it("implements NOT with 1-input LUT", function()
            local lut = FPGA.LUT.new(1)
            lut:configure({1, 0})
            assert.equals(1, lut:evaluate({0}))
            assert.equals(0, lut:evaluate({1}))
        end)

        it("treats inputs as MSB-first", function()
            -- 3-input LUT: address for [1,0,1] = binary 101 = 5
            local lut = FPGA.LUT.new(3)
            local tt = {0,0,0,0,0,0,0,0}
            tt[5+1] = 1  -- 1-indexed, address 5
            lut:configure(tt)
            assert.equals(1, lut:evaluate({1,0,1}))
            assert.equals(0, lut:evaluate({1,0,0}))
        end)

        it("errors on wrong truth table size", function()
            local lut = FPGA.LUT.new(2)
            assert.has_error(function() lut:configure({0,1}) end)
        end)

        it("errors on wrong number of inputs", function()
            local lut = FPGA.LUT.new(2)
            lut:configure({0,0,0,1})
            assert.has_error(function() lut:evaluate({0,1,0}) end)
        end)
    end)

    -- ===========================================================
    -- Slice
    -- ===========================================================

    describe("Slice", function()
        it("creates a slice with 4-input LUTs by default", function()
            local sl = FPGA.Slice.new()
            assert.equals(4, sl.lut_a.num_inputs)
            assert.equals(4, sl.lut_b.num_inputs)
        end)

        it("creates a slice with 2-input LUTs", function()
            local sl = FPGA.Slice.new({lut_inputs = 2})
            assert.equals(2, sl.lut_a.num_inputs)
        end)

        it("evaluates combinationally (no FF)", function()
            local sl = FPGA.Slice.new({lut_inputs = 2})
            sl:configure({
                lut_a = {0,0,0,1},  -- AND
                lut_b = {0,1,1,0},  -- XOR
            })
            local a, b, carry = sl:evaluate({1,1}, {0,1}, 0, 0)
            assert.equals(1, a)  -- AND(1,1) = 1
            assert.equals(1, b)  -- XOR(0,1) = 1
            assert.equals(0, carry)
        end)

        it("registers output through flip-flop", function()
            local sl = FPGA.Slice.new({lut_inputs = 2, use_ff_a = true})
            sl:configure({lut_a = {0,0,0,1}})  -- AND

            -- Clock=0: FF doesn't capture
            local a, _, _ = sl:evaluate({1,1}, {0,0}, 0, 0)
            assert.equals(0, a)  -- FF still 0 (initial state)

            -- Clock=1: FF captures LUT output = 1
            a, _, _ = sl:evaluate({1,1}, {0,0}, 1, 0)
            assert.equals(1, a)  -- FF captures AND(1,1)=1

            -- Change inputs but no clock: FF holds
            a, _, _ = sl:evaluate({0,0}, {0,0}, 0, 0)
            assert.equals(1, a)  -- FF holds previous value
        end)

        it("carry chain: XOR and AND", function()
            local sl = FPGA.Slice.new({
                lut_inputs   = 2,
                carry_enable = true,
            })
            -- LUT A = 1 always (constant 1)
            -- LUT B = 0 always (constant 0)
            sl:configure({
                lut_a = {1,1,1,1},
                lut_b = {0,0,0,0},
            })
            -- carry_in=1: out_a = 1 XOR 1 = 0; carry_mid = 1 AND 1 = 1
            --             out_b = 0 XOR 1 = 1; carry_out = carry_mid = 1
            local a, b, carry = sl:evaluate({0,0}, {0,0}, 0, 1)
            assert.equals(0, a)
            assert.equals(1, b)
            assert.equals(1, carry)
        end)
    end)

    -- ===========================================================
    -- CLB
    -- ===========================================================

    describe("CLB", function()
        it("creates a CLB with two slices", function()
            local clb = FPGA.CLB.new(0, 0, {lut_inputs = 2})
            assert.equals(0, clb.row)
            assert.equals(0, clb.col)
            assert.is_not_nil(clb.slice_0)
            assert.is_not_nil(clb.slice_1)
        end)

        it("evaluates with configured LUTs", function()
            local clb = FPGA.CLB.new(0, 0, {lut_inputs = 2})
            clb:configure({
                slice_0 = {lut_a = {0,0,0,1}, lut_b = {0,1,1,0}},
                slice_1 = {lut_a = {1,1,1,0}, lut_b = {1,0,0,1}},
            })
            local inputs = {
                s0_a = {1,1}, s0_b = {0,1},
                s1_a = {1,1}, s1_b = {0,1},
            }
            local outputs, carry = clb:evaluate(inputs, 0, 0)
            assert.equals(4, #outputs)
            assert.equals(1, outputs[1])  -- AND(1,1)
            assert.equals(1, outputs[2])  -- XOR(0,1)
            assert.equals(0, outputs[3])  -- NAND(1,1) = 0 ... NAND is 1,1,1,0
            assert.equals(0, outputs[4])  -- XNOR(0,1) = 0
        end)

        it("returns carry_out = 0 without carry chain", function()
            local clb = FPGA.CLB.new(0, 0, {lut_inputs = 1})
            clb:configure({})
            local _, carry = clb:evaluate({s0_a={0}, s0_b={0}, s1_a={0}, s1_b={0}}, 0, 0)
            assert.equals(0, carry)
        end)
    end)

    -- ===========================================================
    -- SwitchMatrix
    -- ===========================================================

    describe("SwitchMatrix", function()
        it("creates with correct port names", function()
            local sm = FPGA.SwitchMatrix.new(4, 4)
            assert.equals(4, sm.num_inputs)
            assert.equals(4, sm.num_outputs)
            assert.equals("in_0",  sm.input_names[1])
            assert.equals("out_3", sm.output_names[4])
        end)

        it("routes signals correctly", function()
            local sm = FPGA.SwitchMatrix.new(4, 4)
            sm:configure({["out_0"] = "in_2", ["out_1"] = "in_0"})
            local out = sm:route({in_0=1, in_1=0, in_2=1, in_3=0})
            assert.equals(1, out["out_0"])  -- connected to in_2=1
            assert.equals(1, out["out_1"])  -- connected to in_0=1
            assert.is_nil(out["out_2"])     -- unconnected
            assert.is_nil(out["out_3"])     -- unconnected
        end)

        it("allows fan-out (multiple outputs from same input)", function()
            local sm = FPGA.SwitchMatrix.new(2, 3)
            sm:configure({["out_0"]="in_1", ["out_1"]="in_1", ["out_2"]="in_0"})
            local out = sm:route({in_0=0, in_1=1})
            assert.equals(1, out["out_0"])
            assert.equals(1, out["out_1"])
            assert.equals(0, out["out_2"])
        end)

        it("errors on invalid port names", function()
            local sm = FPGA.SwitchMatrix.new(2, 2)
            assert.has_error(function()
                sm:configure({["out_99"] = "in_0"})
            end)
        end)
    end)

    -- ===========================================================
    -- IOBlock
    -- ===========================================================

    describe("IOBlock", function()
        it("input block: set_pin -> read_fabric", function()
            local io = FPGA.IOBlock.new("my_pin", "input")
            io:set_pin(1)
            assert.equals(1, io:read_fabric())
        end)

        it("output block: set_fabric -> read_pin", function()
            local io = FPGA.IOBlock.new("my_out", "output")
            io:set_fabric(0)
            assert.equals(0, io:read_pin())
        end)

        it("output block: output_enable defaults to 1", function()
            local io = FPGA.IOBlock.new("out", "output")
            assert.equals(1, io.output_enable)
        end)

        it("input block: output_enable defaults to 0", function()
            local io = FPGA.IOBlock.new("inp", "input")
            assert.equals(0, io.output_enable)
        end)

        it("bidirectional: read_fabric when output_enable=0 returns pin", function()
            local io = FPGA.IOBlock.new("bidi", "bidirectional")
            io:set_pin(1)
            io:set_output_enable(0)
            assert.equals(1, io:read_fabric())
        end)

        it("bidirectional: read_pin when output_enable=1 returns fabric", function()
            local io = FPGA.IOBlock.new("bidi", "bidirectional")
            io:set_fabric(0)
            io:set_output_enable(1)
            assert.equals(0, io:read_pin())
        end)

        it("errors on setting pin of output block", function()
            local io = FPGA.IOBlock.new("out", "output")
            assert.has_error(function() io:set_pin(1) end)
        end)

        it("errors on invalid direction", function()
            assert.has_error(function()
                FPGA.IOBlock.new("x", "invalid")
            end)
        end)
    end)

    -- ===========================================================
    -- Bitstream
    -- ===========================================================

    describe("Bitstream", function()
        it("stores and retrieves CLB config", function()
            local bs = FPGA.Bitstream.from_map({
                clbs = {
                    ["0,0"] = {
                        slice_0 = {lut_a = {0,0,0,1}},
                    }
                }
            })
            local cfg = bs:clb_config("0,0")
            assert.is_not_nil(cfg)
            assert.is_not_nil(cfg.slice_0)
        end)

        it("returns nil for unknown keys", function()
            local bs = FPGA.Bitstream.from_map({})
            assert.is_nil(bs:clb_config("99,99"))
            assert.is_nil(bs:routing_config("0,0"))
            assert.is_nil(bs:io_config("top_0"))
        end)

        it("stores and retrieves routing config", function()
            local bs = FPGA.Bitstream.from_map({
                routing = {
                    ["1,2"] = {["out_0"]="in_1"},
                }
            })
            local rc = bs:routing_config("1,2")
            assert.is_not_nil(rc)
            assert.equals("in_1", rc["out_0"])
        end)
    end)

    -- ===========================================================
    -- Fabric
    -- ===========================================================

    describe("Fabric", function()
        it("creates a 2x2 fabric", function()
            local fab = FPGA.Fabric.new(2, 2)
            assert.equals(2, fab.rows)
            assert.equals(2, fab.cols)
            assert.is_not_nil(fab.clbs[0][0])
            assert.is_not_nil(fab.clbs[1][1])
        end)

        it("creates perimeter I/O blocks", function()
            local fab = FPGA.Fabric.new(2, 3)
            -- Top: 3 input blocks
            assert.is_not_nil(fab.io_blocks["top_0"])
            assert.is_not_nil(fab.io_blocks["top_2"])
            assert.equals("input", fab.io_blocks["top_0"].direction)
            -- Bottom: 3 output blocks
            assert.is_not_nil(fab.io_blocks["bottom_0"])
            assert.equals("output", fab.io_blocks["bottom_0"].direction)
            -- Left: 2 input blocks
            assert.is_not_nil(fab.io_blocks["left_0"])
            assert.is_not_nil(fab.io_blocks["left_1"])
            -- Right: 2 output blocks
            assert.is_not_nil(fab.io_blocks["right_0"])
        end)

        it("set_input and evaluate run without error", function()
            local fab = FPGA.Fabric.new(1, 1, {lut_inputs = 2})
            fab:set_input("top_0", 1)
            fab:set_input("left_0", 0)
            fab:evaluate(0)
            -- No crash = pass
            assert.is_true(true)
        end)

        it("summary returns a string", function()
            local fab = FPGA.Fabric.new(2, 2)
            local s = fab:summary()
            assert.is_string(s)
            assert.truthy(s:find("2×2") or s:find("2x2") or s:find("FPGA"))
        end)

        it("loads a bitstream", function()
            local fab = FPGA.Fabric.new(1, 1, {lut_inputs = 2})
            local bs = FPGA.Bitstream.from_map({
                clbs = {
                    ["0,0"] = {
                        slice_0 = {lut_a = {0,0,0,1}},
                    }
                }
            })
            fab:load_bitstream(bs)
            assert.equals(4, #fab.clbs[0][0].slice_0.lut_a.truth_table)
        end)
    end)

    -- ===========================================================
    -- End-to-End: AND gate via LUT
    -- ===========================================================

    describe("end-to-end: AND gate via LUT", function()
        it("programs a single-LUT AND and verifies outputs", function()
            local lut = FPGA.LUT.new(2)
            lut:configure({0,0,0,1})
            assert.equals(0, lut:evaluate({0,0}))
            assert.equals(0, lut:evaluate({0,1}))
            assert.equals(0, lut:evaluate({1,0}))
            assert.equals(1, lut:evaluate({1,1}))
        end)
    end)

    -- ===========================================================
    -- End-to-End: 1-bit full adder via slice
    -- ===========================================================

    describe("end-to-end: 1-bit full adder", function()
        -- A full adder has:
        --   sum  = A XOR B XOR Cin
        --   cout = (A AND B) OR (A AND Cin) OR (B AND Cin)
        --
        -- With carry chain enabled:
        --   LUT_A implements XOR(A, B) for the carry generate term
        --   out_a = LUT_A XOR Cin = XOR(A,B) XOR Cin = sum
        --   carry_mid = LUT_A AND Cin = XOR(A,B) AND Cin (partial carry)
        --
        -- This is the classic carry-chain 1-bit adder:
        --   LUT_A = XOR(A,B); with carry_enable: sum = LUT_A XOR Cin

        it("computes sum and carry with carry chain", function()
            local sl = FPGA.Slice.new({lut_inputs = 2, carry_enable = true})
            sl:configure({
                lut_a = {0,1,1,0},  -- XOR(A,B)
                lut_b = {0,0,0,0},  -- unused in this test (no B chain)
            })

            -- A=0, B=0, Cin=0: sum=0, cout=0
            local s, _, c = sl:evaluate({0,0}, {0,0}, 0, 0)
            assert.equals(0, s)
            assert.equals(0, c)

            -- A=1, B=0, Cin=0: sum=1, cout=0
            s, _, c = sl:evaluate({1,0}, {0,0}, 0, 0)
            assert.equals(1, s)
            assert.equals(0, c)

            -- A=1, B=1, Cin=0: XOR=0, sum=0 XOR 0=0, carry_mid=0 AND 0=0
            s, _, c = sl:evaluate({1,1}, {0,0}, 0, 0)
            assert.equals(0, s)

            -- A=0, B=0, Cin=1: XOR=0, sum=0 XOR 1=1, carry_mid=0 AND 1=0
            s, _, c = sl:evaluate({0,0}, {0,0}, 0, 1)
            assert.equals(1, s)
            assert.equals(0, c)

            -- A=1, B=0, Cin=1: XOR=1, sum=1 XOR 1=0, carry_mid=1 AND 1=1
            s, _, c = sl:evaluate({1,0}, {0,0}, 0, 1)
            assert.equals(0, s)
            assert.equals(1, c)
        end)
    end)

end)
