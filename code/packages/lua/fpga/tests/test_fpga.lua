-- Tests for coding_adventures.fpga
-- Covers LUT, Slice, CLB, SwitchMatrix, IOBlock, Fabric, and Bitstream.

local fpga         = require("coding_adventures.fpga")
local LUT          = fpga.LUT
local Slice        = fpga.Slice
local CLB          = fpga.CLB
local SwitchMatrix = fpga.SwitchMatrix
local IOBlock      = fpga.IOBlock
local Fabric       = fpga.Fabric
local Bitstream    = fpga.Bitstream

-- ===========================================================================
-- LUT Tests
-- ===========================================================================

describe("LUT", function()
  it("creates with correct defaults", function()
    local lut = LUT.new(4)
    assert.equals(4, lut.num_inputs)
    assert.equals(16, #lut.truth_table)
    for _, v in ipairs(lut.truth_table) do
      assert.equals(0, v)
    end
  end)

  it("creates 2-input LUT", function()
    local lut = LUT.new(2)
    assert.equals(2, lut.num_inputs)
    assert.equals(4, #lut.truth_table)
  end)

  it("configures AND gate", function()
    local lut = LUT.new(2)
    lut:configure({0, 0, 0, 1})
    assert.same({0, 0, 0, 1}, lut.truth_table)
  end)

  it("configures OR gate", function()
    local lut = LUT.new(2)
    lut:configure({0, 1, 1, 1})
    assert.same({0, 1, 1, 1}, lut.truth_table)
  end)

  it("configures XOR gate", function()
    local lut = LUT.new(2)
    lut:configure({0, 1, 1, 0})
    assert.same({0, 1, 1, 0}, lut.truth_table)
  end)

  it("evaluates AND correctly", function()
    local lut = LUT.new(2)
    lut:configure({0, 0, 0, 1})
    assert.equals(0, lut:evaluate({0, 0}))
    assert.equals(0, lut:evaluate({0, 1}))
    assert.equals(0, lut:evaluate({1, 0}))
    assert.equals(1, lut:evaluate({1, 1}))
  end)

  it("evaluates OR correctly", function()
    local lut = LUT.new(2)
    lut:configure({0, 1, 1, 1})
    assert.equals(0, lut:evaluate({0, 0}))
    assert.equals(1, lut:evaluate({0, 1}))
    assert.equals(1, lut:evaluate({1, 0}))
    assert.equals(1, lut:evaluate({1, 1}))
  end)

  it("evaluates XOR correctly", function()
    local lut = LUT.new(2)
    lut:configure({0, 1, 1, 0})
    assert.equals(0, lut:evaluate({0, 0}))
    assert.equals(1, lut:evaluate({0, 1}))
    assert.equals(1, lut:evaluate({1, 0}))
    assert.equals(0, lut:evaluate({1, 1}))
  end)

  it("evaluates 4-input LUT", function()
    -- NAND: 1 only when NOT all inputs are 1
    local lut = LUT.new(4)
    local tt = {}
    for i = 0, 15 do
      -- NAND: output is 0 only when all 4 bits of i are 1 (i == 15)
      tt[i + 1] = (i == 15) and 0 or 1
    end
    lut:configure(tt)
    assert.equals(1, lut:evaluate({0, 0, 0, 0}))
    assert.equals(1, lut:evaluate({1, 1, 1, 0}))
    assert.equals(0, lut:evaluate({1, 1, 1, 1}))
  end)

  it("rejects wrong truth table size", function()
    local lut = LUT.new(2)
    assert.has_error(function()
      lut:configure({0, 0, 0})  -- needs 4, got 3
    end)
  end)

  it("rejects wrong number of inputs", function()
    local lut = LUT.new(2)
    lut:configure({0, 0, 0, 1})
    assert.has_error(function()
      lut:evaluate({0, 0, 0})  -- needs 2, got 3
    end)
  end)

  it("rejects non-bit truth table entries", function()
    local lut = LUT.new(2)
    assert.has_error(function()
      lut:configure({0, 0, 0, 2})
    end)
  end)
end)

-- ===========================================================================
-- Slice Tests
-- ===========================================================================

describe("Slice", function()
  it("creates with defaults", function()
    local s = Slice.new()
    assert.equals(4, s.lut_a.num_inputs)
    assert.equals(4, s.lut_b.num_inputs)
    assert.is_false(s.use_ff_a)
    assert.is_false(s.use_ff_b)
    assert.is_false(s.carry_enable)
  end)

  it("creates with custom options", function()
    local s = Slice.new({ lut_inputs = 2, use_ff_a = true, carry_enable = true })
    assert.equals(2, s.lut_a.num_inputs)
    assert.is_true(s.use_ff_a)
    assert.is_true(s.carry_enable)
  end)

  it("configures LUT A", function()
    local s = Slice.new({ lut_inputs = 2 })
    s:configure({ lut_a = {0, 0, 0, 1} })
    assert.same({0, 0, 0, 1}, s.lut_a.truth_table)
  end)

  it("configures both LUTs", function()
    local s = Slice.new({ lut_inputs = 2 })
    s:configure({ lut_a = {0, 0, 0, 1}, lut_b = {0, 1, 1, 0} })
    assert.same({0, 0, 0, 1}, s.lut_a.truth_table)
    assert.same({0, 1, 1, 0}, s.lut_b.truth_table)
  end)

  it("evaluates combinational (no FF, no carry)", function()
    local s = Slice.new({ lut_inputs = 2 })
    s:configure({ lut_a = {0, 0, 0, 1}, lut_b = {0, 1, 1, 0} })
    local out_a, out_b, carry = s:evaluate({1, 1}, {0, 1}, 0, 0)
    assert.equals(1, out_a)   -- AND(1,1) = 1
    assert.equals(1, out_b)   -- XOR(0,1) = 1
    assert.equals(0, carry)
  end)

  it("evaluates with carry chain", function()
    -- Carry chain: out_a = lut_a XOR carry_in, carry_mid = lut_a AND carry_in
    -- With lut_a = 1 (OR(1,0) = 1), carry_in = 1:
    --   out_a = 1 XOR 1 = 0
    --   carry_mid = 1 AND 1 = 1
    -- With lut_b = 0 (AND(0,0) = 0), carry_mid = 1:
    --   out_b = 0 XOR 1 = 1
    --   carry_out = 0 AND 1 = 0
    local s = Slice.new({ lut_inputs = 2, carry_enable = true })
    s:configure({ lut_a = {0, 1, 1, 1}, lut_b = {0, 0, 0, 1} })
    local out_a, out_b, carry_out = s:evaluate({1, 0}, {0, 0}, 0, 1)
    assert.equals(0, out_a)      -- 1 XOR 1 = 0
    assert.equals(1, out_b)      -- 0 XOR 1 = 1
    assert.equals(0, carry_out)  -- 0 AND 1 = 0
  end)

  it("evaluates with flip-flop A (captures on rising edge)", function()
    local s = Slice.new({ lut_inputs = 2, use_ff_a = true })
    s:configure({ lut_a = {0, 0, 0, 1} })  -- AND

    -- Clock=0: LUT computes 1 (AND(1,1)) but FF doesn't capture yet
    local out_a, _, _ = s:evaluate({1, 1}, {0, 0}, 0, 0)
    assert.equals(0, out_a)   -- FF still holds reset value 0

    -- Clock=1: FF captures the LUT output
    out_a, _, _ = s:evaluate({1, 1}, {0, 0}, 1, 0)
    assert.equals(1, out_a)   -- FF captured 1
  end)

  it("flip-flop holds value between clocks", function()
    local s = Slice.new({ lut_inputs = 2, use_ff_a = true })
    s:configure({ lut_a = {0, 0, 0, 1} })  -- AND

    -- Clock=1: capture AND(1,1) = 1
    s:evaluate({1, 1}, {0, 0}, 1, 0)

    -- Clock=0 with AND(0,0) = 0: FF holds previous value 1
    local out_a, _, _ = s:evaluate({0, 0}, {0, 0}, 0, 0)
    assert.equals(1, out_a)   -- FF holds 1
  end)
end)

-- ===========================================================================
-- CLB Tests
-- ===========================================================================

describe("CLB", function()
  it("creates at given position", function()
    local clb = CLB.new(2, 3)
    assert.equals(2, clb.row)
    assert.equals(3, clb.col)
  end)

  it("creates with custom lut_inputs", function()
    local clb = CLB.new(0, 0, { lut_inputs = 2 })
    assert.equals(2, clb.slice_0.lut_a.num_inputs)
    assert.equals(2, clb.slice_1.lut_a.num_inputs)
  end)

  it("configures slice_0 LUTs", function()
    local clb = CLB.new(0, 0, { lut_inputs = 2 })
    clb:configure({ slice_0 = { lut_a = {0, 0, 0, 1}, lut_b = {0, 1, 1, 0} } })
    assert.same({0, 0, 0, 1}, clb.slice_0.lut_a.truth_table)
    assert.same({0, 1, 1, 0}, clb.slice_0.lut_b.truth_table)
  end)

  it("configures both slices", function()
    local clb = CLB.new(0, 0, { lut_inputs = 2 })
    clb:configure({
      slice_0 = { lut_a = {0, 0, 0, 1} },
      slice_1 = { lut_a = {1, 1, 1, 0} },
    })
    assert.same({0, 0, 0, 1}, clb.slice_0.lut_a.truth_table)
    assert.same({1, 1, 1, 0}, clb.slice_1.lut_a.truth_table)
  end)

  it("evaluates and returns 4 outputs", function()
    local clb = CLB.new(0, 0, { lut_inputs = 2 })
    clb:configure({
      slice_0 = { lut_a = {0, 0, 0, 1}, lut_b = {0, 1, 1, 0} },
      slice_1 = { lut_a = {1, 1, 1, 0}, lut_b = {1, 0, 0, 1} },
    })
    local inputs = {
      s0_a = {1, 1},
      s0_b = {0, 1},
      s1_a = {1, 1},
      s1_b = {0, 0},
    }
    local outputs, carry_out = clb:evaluate(inputs, 0, 0)
    assert.equals(4, #outputs)
    assert.equals(1, outputs[1])   -- AND(1,1) = 1
    assert.equals(1, outputs[2])   -- XOR(0,1) = 1
    assert.equals(0, outputs[3])   -- NAND(1,1) = 0
    assert.equals(1, outputs[4])   -- XNOR(0,0) = 1
    assert.equals(0, carry_out)
  end)
end)

-- ===========================================================================
-- SwitchMatrix Tests
-- ===========================================================================

describe("SwitchMatrix", function()
  it("creates with correct port counts", function()
    local sm = SwitchMatrix.new(4, 4)
    assert.equals(4, sm.num_inputs)
    assert.equals(4, sm.num_outputs)
    assert.equals(4, #sm.input_names)
    assert.equals(4, #sm.output_names)
  end)

  it("generates correct port names", function()
    local sm = SwitchMatrix.new(3, 2)
    assert.same({"in_0", "in_1", "in_2"}, sm.input_names)
    assert.same({"out_0", "out_1"}, sm.output_names)
  end)

  it("configures connections", function()
    local sm = SwitchMatrix.new(4, 4)
    sm:configure({ out_0 = "in_2", out_1 = "in_0" })
    assert.equals("in_2", sm.connections["out_0"])
    assert.equals("in_0", sm.connections["out_1"])
  end)

  it("routes signals through connections", function()
    local sm = SwitchMatrix.new(4, 4)
    sm:configure({ out_0 = "in_2" })
    local result = sm:route({ in_0 = 0, in_1 = 0, in_2 = 1, in_3 = 0 })
    assert.equals(1, result["out_0"])
  end)

  it("unconnected outputs produce nil", function()
    local sm = SwitchMatrix.new(4, 4)
    sm:configure({ out_0 = "in_0" })
    local result = sm:route({ in_0 = 1, in_1 = 0, in_2 = 0, in_3 = 0 })
    assert.is_nil(result["out_1"])
    assert.is_nil(result["out_2"])
    assert.is_nil(result["out_3"])
  end)

  it("fan-out: multiple outputs from same input", function()
    local sm = SwitchMatrix.new(4, 4)
    sm:configure({ out_0 = "in_2", out_1 = "in_2", out_2 = "in_2" })
    local result = sm:route({ in_0 = 0, in_1 = 0, in_2 = 1, in_3 = 0 })
    assert.equals(1, result["out_0"])
    assert.equals(1, result["out_1"])
    assert.equals(1, result["out_2"])
  end)

  it("rejects invalid output port name", function()
    local sm = SwitchMatrix.new(4, 4)
    assert.has_error(function()
      sm:configure({ out_99 = "in_0" })
    end)
  end)

  it("rejects invalid input port name", function()
    local sm = SwitchMatrix.new(4, 4)
    assert.has_error(function()
      sm:configure({ out_0 = "in_99" })
    end)
  end)
end)

-- ===========================================================================
-- IOBlock Tests
-- ===========================================================================

describe("IOBlock", function()
  it("creates input block", function()
    local io = IOBlock.new("pin_0", "input")
    assert.equals("pin_0", io.name)
    assert.equals("input", io.direction)
    assert.equals(0, io.output_enable)
  end)

  it("creates output block", function()
    local io = IOBlock.new("pin_0", "output")
    assert.equals("output", io.direction)
    assert.equals(1, io.output_enable)  -- always enabled for output
  end)

  it("creates bidirectional block", function()
    local io = IOBlock.new("pin_0", "bidirectional")
    assert.equals("bidirectional", io.direction)
    assert.equals(0, io.output_enable)  -- starts as input
  end)

  it("rejects invalid direction", function()
    assert.has_error(function()
      IOBlock.new("pin_0", "invalid")
    end)
  end)

  it("input block: set_pin and read_fabric", function()
    local io = IOBlock.new("pin_0", "input")
    io:set_pin(1)
    assert.equals(1, io:read_fabric())
    assert.equals(1, io:read_pin())
  end)

  it("output block: set_fabric and read_pin", function()
    local io = IOBlock.new("pin_0", "output")
    io:set_fabric(1)
    assert.equals(1, io:read_pin())
    assert.equals(1, io:read_fabric())
  end)

  it("input block rejects set_fabric", function()
    local io = IOBlock.new("pin_0", "input")
    assert.has_error(function()
      io:set_fabric(1)
    end)
  end)

  it("output block rejects set_pin", function()
    local io = IOBlock.new("pin_0", "output")
    assert.has_error(function()
      io:set_pin(1)
    end)
  end)

  it("bidirectional: OE=0 reads pin (input mode)", function()
    local io = IOBlock.new("pin_0", "bidirectional")
    io:set_pin(1)
    assert.equals(1, io:read_fabric())
    assert.equals(1, io:read_pin())
  end)

  it("bidirectional: OE=1 drives pin (output mode)", function()
    local io = IOBlock.new("pin_0", "bidirectional")
    io:set_fabric(0)  -- need to set fabric before enabling output
    io:set_output_enable(1)
    io:set_fabric(1)
    assert.equals(1, io:read_pin())
    assert.equals(1, io:read_fabric())
  end)

  it("set_output_enable rejects non-bidirectional", function()
    local io_in = IOBlock.new("pin_0", "input")
    assert.has_error(function()
      io_in:set_output_enable(1)
    end)
    local io_out = IOBlock.new("pin_0", "output")
    assert.has_error(function()
      io_out:set_output_enable(0)
    end)
  end)

  it("read_fabric returns nil when nothing set", function()
    local io = IOBlock.new("pin_0", "input")
    assert.is_nil(io:read_fabric())
  end)
end)

-- ===========================================================================
-- Bitstream Tests
-- ===========================================================================

describe("Bitstream", function()
  it("parses from empty map", function()
    local bs = Bitstream.from_map({})
    assert.is_nil(bs:clb_config("0_0"))
    assert.is_nil(bs:routing_config("0_0"))
    assert.is_nil(bs:io_config("pin_0"))
  end)

  it("parses CLB config", function()
    local bs = Bitstream.from_map({
      clbs = {
        ["0_0"] = {
          slice_0 = { lut_a = {0, 0, 0, 1} }
        }
      }
    })
    local cfg = bs:clb_config("0_0")
    assert.is_not_nil(cfg)
    assert.same({0, 0, 0, 1}, cfg.slice_0.lut_a)
  end)

  it("parses routing config", function()
    local bs = Bitstream.from_map({
      routing = {
        ["0_0"] = { out_0 = "in_2" }
      }
    })
    local cfg = bs:routing_config("0_0")
    assert.equals("in_2", cfg["out_0"])
  end)

  it("parses IO config", function()
    local bs = Bitstream.from_map({
      io = {
        top_0 = { direction = "input" },
        bottom_0 = { direction = "output" },
      }
    })
    assert.equals("input", bs:io_config("top_0").direction)
    assert.equals("output", bs:io_config("bottom_0").direction)
  end)

  it("returns nil for missing keys", function()
    local bs = Bitstream.from_map({
      clbs    = { ["0_0"] = {} },
      routing = {},
      io      = {},
    })
    assert.is_nil(bs:clb_config("1_1"))
    assert.is_nil(bs:routing_config("0_0"))
    assert.is_nil(bs:io_config("pin_x"))
  end)
end)

-- ===========================================================================
-- Fabric Tests
-- ===========================================================================

describe("Fabric", function()
  it("creates 2x2 fabric", function()
    local f = Fabric.new(2, 2)
    assert.equals(2, f.rows)
    assert.equals(2, f.cols)
  end)

  it("creates correct number of CLBs", function()
    local f = Fabric.new(2, 3)
    local count = 0
    for _ in pairs(f.clbs) do count = count + 1 end
    assert.equals(6, count)
  end)

  it("creates correct number of switch matrices", function()
    local f = Fabric.new(2, 3)
    local count = 0
    for _ in pairs(f.switch_matrices) do count = count + 1 end
    assert.equals(6, count)
  end)

  it("creates perimeter I/O blocks for 2x2", function()
    local f = Fabric.new(2, 2)
    -- top_0, top_1, bottom_0, bottom_1, left_0, left_1, right_0, right_1 = 8 total
    local count = 0
    for _ in pairs(f.io_blocks) do count = count + 1 end
    assert.equals(8, count)
  end)

  it("top IO is input, bottom IO is output", function()
    local f = Fabric.new(2, 2)
    assert.equals("input",  f.io_blocks["top_0"].direction)
    assert.equals("output", f.io_blocks["bottom_0"].direction)
    assert.equals("input",  f.io_blocks["left_0"].direction)
    assert.equals("output", f.io_blocks["right_0"].direction)
  end)

  it("summary returns correct counts", function()
    local f = Fabric.new(2, 2)
    local s = f:summary()
    assert.equals(2,  s.rows)
    assert.equals(2,  s.cols)
    assert.equals(4,  s.clb_count)
    assert.equals(16, s.lut_count)
    assert.equals(16, s.ff_count)
    assert.equals(4,  s.switch_matrix_count)
    assert.equals(8,  s.io_block_count)
  end)

  it("set_input and read_output", function()
    local f = Fabric.new(2, 2)
    f:set_input("top_0", 1)
    assert.equals(1, f.io_blocks["top_0"]:read_fabric())
  end)

  it("set_input rejects unknown pin", function()
    local f = Fabric.new(2, 2)
    assert.has_error(function()
      f:set_input("nonexistent", 1)
    end)
  end)

  it("loads bitstream and configures CLB", function()
    local f = Fabric.new(1, 1, { lut_inputs = 2 })
    local bs = Bitstream.from_map({
      clbs = {
        ["0_0"] = {
          slice_0 = { lut_a = {0, 0, 0, 1} }
        }
      },
      routing = {},
      io = {},
    })
    f:load_bitstream(bs)
    assert.same({0, 0, 0, 1}, f.clbs["0_0"].slice_0.lut_a.truth_table)
  end)

  it("loads bitstream and configures routing", function()
    local f = Fabric.new(1, 1, { switch_size = 4 })
    local bs = Bitstream.from_map({
      clbs = {},
      routing = {
        ["0_0"] = { out_0 = "in_1" }
      },
      io = {},
    })
    f:load_bitstream(bs)
    assert.equals("in_1", f.switch_matrices["0_0"].connections["out_0"])
  end)

  it("loads bitstream and reconfigures IO direction", function()
    local f = Fabric.new(1, 1)
    local bs = Bitstream.from_map({
      clbs = {},
      routing = {},
      io = {
        top_0 = { direction = "bidirectional" }
      },
    })
    f:load_bitstream(bs)
    assert.equals("bidirectional", f.io_blocks["top_0"].direction)
  end)

  it("evaluate runs without error", function()
    local f = Fabric.new(2, 2)
    f:evaluate(0)
    f:evaluate(1)
    -- No error = pass
  end)
end)

-- ===========================================================================
-- End-to-End: AND gate programmed onto FPGA
-- ===========================================================================

describe("End-to-end: AND gate on FPGA", function()
  it("programs AND gate and reads output through CLB", function()
    -- Build a 1x1 FPGA with 2-input LUTs
    local f = Fabric.new(1, 1, { lut_inputs = 2 })

    -- Program AND truth table onto slice_0 lut_a
    local bs = Bitstream.from_map({
      clbs = {
        ["0_0"] = {
          slice_0 = { lut_a = {0, 0, 0, 1} }
        }
      },
      routing = {},
      io = {},
    })
    f:load_bitstream(bs)

    -- Verify the LUT directly
    local clb = f.clbs["0_0"]
    assert.equals(0, clb.slice_0.lut_a:evaluate({0, 0}))
    assert.equals(0, clb.slice_0.lut_a:evaluate({0, 1}))
    assert.equals(0, clb.slice_0.lut_a:evaluate({1, 0}))
    assert.equals(1, clb.slice_0.lut_a:evaluate({1, 1}))
  end)
end)

-- ===========================================================================
-- End-to-End: XOR gate with carry (1-bit full adder per slice)
-- ===========================================================================

describe("End-to-end: 1-bit full adder in a slice", function()
  it("implements sum bit via XOR and carry via AND", function()
    -- A full adder computes:
    --   sum   = a XOR b XOR cin
    --   carry = (a AND b) OR (cin AND (a XOR b))
    --
    -- For a simple test: lut_a = XOR(a,b), carry_enable = true
    -- With a=1, b=1, cin=0:
    --   lut_a result = XOR(1,1) = 0
    --   out_a = 0 XOR 0 = 0  (sum)
    --   carry_mid = 0 AND 0 = 0

    local s = Slice.new({ lut_inputs = 2, carry_enable = true })
    s:configure({
      lut_a = {0, 1, 1, 0},   -- XOR
      lut_b = {0, 0, 0, 0},   -- all zeros
    })

    -- a=1, b=1, cin=0
    local sum, _, carry = s:evaluate({1, 1}, {0, 0}, 0, 0)
    assert.equals(0, sum)    -- XOR(1,1) XOR 0 = 0
    assert.equals(0, carry)  -- XOR(1,1) AND 0 = 0

    -- a=1, b=0, cin=1
    sum, _, carry = s:evaluate({1, 0}, {0, 0}, 0, 1)
    assert.equals(0, sum)    -- XOR(1,0) XOR 1 = 1 XOR 1 = 0
    assert.equals(1, carry)  -- XOR(1,0) AND 1 = 1 AND 1 = 1
  end)
end)
