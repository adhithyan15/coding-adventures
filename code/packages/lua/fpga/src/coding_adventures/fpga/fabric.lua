-- fabric.lua — FPGA Fabric: the full grid of CLBs, switch matrices, and I/O blocks
--
-- The fabric is a rows × cols grid of CLBs, connected by switch matrices.
-- I/O blocks line the perimeter:
--   Top row:    input blocks  "top_0".."top_{cols-1}"
--   Bottom row: output blocks "bottom_0".."bottom_{cols-1}"
--   Left col:   input blocks  "left_0".."left_{rows-1}"
--   Right col:  output blocks "right_0".."right_{rows-1}"
--
-- evaluate(clock) runs one clock cycle:
--   1. Collect switch matrix output signals from I/O inputs + CLB outputs
--   2. Route signals through each CLB's switch matrix
--   3. Evaluate each CLB
--   4. Drive output I/O blocks from CLB outputs
--
-- In this simplified model, each CLB has one switch matrix that can connect
-- any input or neighbor output to its LUT inputs.

local CLB          = require("coding_adventures.fpga.clb")
local SwitchMatrix = require("coding_adventures.fpga.switch_matrix")
local IOBlock      = require("coding_adventures.fpga.io_block")
local Bitstream    = require("coding_adventures.fpga.bitstream")

local Fabric = {}
Fabric.__index = Fabric

-- Creates a new fabric with the given grid size.
-- Each CLB gets 4 LUT inputs (2 per slice), so each switch matrix has
-- 4 outputs and some number of inputs.
function Fabric.new(rows, cols, opts)
    opts = opts or {}
    local lut_inputs = opts.lut_inputs or 4

    -- CLB grid: clbs[row][col]
    local clbs = {}
    for r = 0, rows - 1 do
        clbs[r] = {}
        for c = 0, cols - 1 do
            clbs[r][c] = CLB.new(r, c, {lut_inputs = lut_inputs})
        end
    end

    -- Switch matrix per CLB (maps some inputs to 4 CLB LUT inputs: s0_a, s0_b, s1_a, s1_b)
    -- We give each switch matrix 8 inputs (global signals) and 4 outputs
    local switch_matrices = {}
    for r = 0, rows - 1 do
        switch_matrices[r] = {}
        for c = 0, cols - 1 do
            switch_matrices[r][c] = SwitchMatrix.new(8, 4)
        end
    end

    -- I/O blocks: perimeter
    local io_blocks = {}

    -- Top: input blocks
    for c = 0, cols - 1 do
        local name = "top_" .. c
        io_blocks[name] = IOBlock.new(name, "input")
    end
    -- Bottom: output blocks
    for c = 0, cols - 1 do
        local name = "bottom_" .. c
        io_blocks[name] = IOBlock.new(name, "output")
    end
    -- Left: input blocks
    for r = 0, rows - 1 do
        local name = "left_" .. r
        io_blocks[name] = IOBlock.new(name, "input")
    end
    -- Right: output blocks
    for r = 0, rows - 1 do
        local name = "right_" .. r
        io_blocks[name] = IOBlock.new(name, "output")
    end

    return setmetatable({
        rows            = rows,
        cols            = cols,
        lut_inputs      = lut_inputs,
        clbs            = clbs,
        switch_matrices = switch_matrices,
        io_blocks       = io_blocks,
        -- CLB output signals from last evaluate
        clb_outputs     = {},
    }, Fabric)
end

-- Loads a bitstream, configuring CLBs, routing, and I/O blocks.
function Fabric:load_bitstream(bs)
    for r = 0, self.rows - 1 do
        for c = 0, self.cols - 1 do
            local key = r .. "," .. c
            local clb_cfg = bs:clb_config(key)
            if clb_cfg then
                self.clbs[r][c]:configure(clb_cfg)
            end
            local routing_cfg = bs:routing_config(key)
            if routing_cfg then
                self.switch_matrices[r][c]:configure(routing_cfg)
            end
        end
    end
    -- Configure I/O blocks if bitstream specifies them
    for pin_name, _ in pairs(self.io_blocks) do
        local io_cfg = bs:io_config(pin_name)
        if io_cfg then
            -- The direction is already set; other config can go here
        end
    end
end

-- Sets an input I/O block's pin value.
function Fabric:set_input(pin_name, value)
    local io = self.io_blocks[pin_name]
    assert(io, "unknown I/O block: " .. tostring(pin_name))
    io:set_pin(value)
end

-- Reads an output I/O block's pin value.
function Fabric:read_output(pin_name)
    local io = self.io_blocks[pin_name]
    assert(io, "unknown I/O block: " .. tostring(pin_name))
    return io:read_pin()
end

-- Evaluates one clock cycle across the entire fabric.
--
-- Signal routing model:
-- The switch matrices have 8 inputs (in_0..in_7) and 4 outputs (out_0..out_3).
-- in_0..in_{cols-1} are top I/O inputs for this column
-- in_{cols}..in_{cols+rows-1} are left I/O inputs for this row
-- in_{cols+rows}..in_7 are previous CLB outputs (if any)
--
-- out_0=s0_a, out_1=s0_b, out_2=s1_a, out_3=s1_b
function Fabric:evaluate(clock)
    -- Build global signal map for switch matrix inputs
    for r = 0, self.rows - 1 do
        for c = 0, self.cols - 1 do
            -- Collect 8 potential input signals
            local sigs = {}
            -- in_0: top input for this column
            sigs["in_0"] = (self.io_blocks["top_" .. c] and
                            self.io_blocks["top_" .. c]:read_fabric()) or 0
            -- in_1: left input for this row
            sigs["in_1"] = (self.io_blocks["left_" .. r] and
                            self.io_blocks["left_" .. r]:read_fabric()) or 0
            -- in_2..in_7: zeros (or neighbor outputs in future extension)
            for i = 2, 7 do sigs["in_" .. i] = 0 end

            -- Route through switch matrix
            local sm = self.switch_matrices[r][c]
            local routed = sm:route(sigs)

            -- Collect LUT inputs from routing
            -- out_0=s0_a first input, out_1=s0_b first input,
            -- out_2=s1_a first input, out_3=s1_b first input
            local function make_inputs(signal_val)
                local v = signal_val or 0
                local arr = {}
                for _ = 1, self.lut_inputs do table.insert(arr, v) end
                return arr
            end

            local clb_inputs = {
                s0_a = make_inputs(routed["out_0"]),
                s0_b = make_inputs(routed["out_1"]),
                s1_a = make_inputs(routed["out_2"]),
                s1_b = make_inputs(routed["out_3"]),
            }

            local outputs, _ = self.clbs[r][c]:evaluate(clb_inputs, clock, 0)
            self.clb_outputs[r .. "," .. c] = outputs
        end
    end

    -- Drive output I/O blocks from first CLB outputs in respective rows/cols
    -- Bottom outputs: driven by CLB (last row, each col) output[0]
    for c = 0, self.cols - 1 do
        local key = (self.rows - 1) .. "," .. c
        local outs = self.clb_outputs[key]
        if outs then
            local io = self.io_blocks["bottom_" .. c]
            if io then
                io:set_fabric(outs[1])
            end
        end
    end
    -- Right outputs: driven by CLB (each row, last col) output[0]
    for r = 0, self.rows - 1 do
        local key = r .. "," .. (self.cols - 1)
        local outs = self.clb_outputs[key]
        if outs then
            local io = self.io_blocks["right_" .. r]
            if io then
                io:set_fabric(outs[1])
            end
        end
    end
end

-- Returns a summary string of the fabric state.
function Fabric:summary()
    local lines = {
        string.format("FPGA Fabric %d×%d", self.rows, self.cols),
        string.format("  CLBs: %d", self.rows * self.cols),
        string.format("  I/O Blocks: %d", (2*self.rows + 2*self.cols)),
    }
    return table.concat(lines, "\n")
end

return Fabric
