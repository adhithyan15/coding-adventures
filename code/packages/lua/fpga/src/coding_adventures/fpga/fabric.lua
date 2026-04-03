--[[
  Fabric — the complete FPGA top-level module.

  ## What is the FPGA Fabric?

  The fabric is the complete FPGA device, tying together all the components:

    - A grid of CLBs (Configurable Logic Blocks) — the logic resources
    - Switch matrices for routing signals between CLBs
    - I/O blocks around the perimeter — external interface

  ## Configuration Flow

      1. Create a fabric with specified grid dimensions.
      2. Load a bitstream (configuration map).
      3. Set input pin values (external signals).
      4. Evaluate (propagate signals through the fabric).
      5. Read output pin values (results).

  ## Grid Layout

  The fabric is organized as a rows x cols grid:

      IO   IO   IO   IO
      IO  CLB  CLB   IO
      IO  CLB  CLB   IO
      IO   IO   IO   IO

  Each CLB position has an associated switch matrix for routing.

  Top edge I/O blocks are inputs; bottom edge are outputs.
  Left edge I/O blocks are inputs; right edge are outputs.

  ## Simplifications

  This model makes several simplifications compared to real FPGAs:
    - Single-cycle evaluation (no routing delay modeling)
    - Switch matrices are per-CLB (real FPGAs have more complex topologies)
    - No global clock tree modeling
    - No DSP blocks or Block RAM tiles
    - Evaluate() does a single pass over CLBs with zero inputs
      (full signal-path tracing requires a complete netlist)
]]

local CLB          = require("coding_adventures.fpga.clb")
local SwitchMatrix = require("coding_adventures.fpga.switch_matrix")
local IOBlock      = require("coding_adventures.fpga.io_block")
local Bitstream    = require("coding_adventures.fpga.bitstream")

local Fabric = {}
Fabric.__index = Fabric

--- Creates a new FPGA fabric with the given grid dimensions.
--
-- Options (table, optional):
--   lut_inputs   (default 4) — number of inputs per LUT
--   switch_size  (default 8) — ports per switch matrix side
--
-- I/O blocks are automatically created around the perimeter:
--   top_0 ... top_{cols-1}    → input
--   bottom_0 ... bottom_{cols-1} → output
--   left_0 ... left_{rows-1}  → input
--   right_0 ... right_{rows-1} → output
--
-- @param rows  number of CLB rows
-- @param cols  number of CLB columns
-- @param opts  optional configuration table
-- @return new Fabric object
function Fabric.new(rows, cols, opts)
  assert(type(rows) == "number" and rows > 0, "rows must be positive")
  assert(type(cols) == "number" and cols > 0, "cols must be positive")
  opts = opts or {}
  local lut_inputs  = opts.lut_inputs  or 4
  local switch_size = opts.switch_size or 8

  -- Create CLB grid (keyed by "row_col")
  local clbs             = {}
  local switch_matrices  = {}
  for r = 0, rows - 1 do
    for c = 0, cols - 1 do
      local key = r .. "_" .. c
      clbs[key]            = CLB.new(r, c, { lut_inputs = lut_inputs })
      switch_matrices[key] = SwitchMatrix.new(switch_size, switch_size)
    end
  end

  -- Create perimeter I/O blocks
  local io_blocks = {}
  for c = 0, cols - 1 do
    local top_name    = "top_" .. c
    local bottom_name = "bottom_" .. c
    io_blocks[top_name]    = IOBlock.new(top_name,    "input")
    io_blocks[bottom_name] = IOBlock.new(bottom_name, "output")
  end
  for r = 0, rows - 1 do
    local left_name  = "left_" .. r
    local right_name = "right_" .. r
    io_blocks[left_name]  = IOBlock.new(left_name,  "input")
    io_blocks[right_name] = IOBlock.new(right_name, "output")
  end

  return setmetatable({
    rows            = rows,
    cols            = cols,
    clbs            = clbs,
    switch_matrices = switch_matrices,
    io_blocks       = io_blocks,
    lut_inputs      = lut_inputs,
  }, Fabric)
end

--- Loads a bitstream configuration into the fabric.
-- Applies CLB, routing, and I/O configurations from the bitstream.
--
-- @param bitstream  a Bitstream object
-- @return self (for chaining)
function Fabric:load_bitstream(bitstream)
  -- Apply CLB configurations
  for key, clb in pairs(self.clbs) do
    local config = bitstream:clb_config(key)
    if config then
      clb:configure(self:_parse_clb_config(config))
    end
  end

  -- Apply routing configurations
  for key, sm in pairs(self.switch_matrices) do
    local config = bitstream:routing_config(key)
    if config then
      sm:configure(config)
    end
  end

  -- Apply I/O configurations
  for name, _ in pairs(self.io_blocks) do
    local config = bitstream:io_config(name)
    if config then
      local direction = config.direction or "input"
      self.io_blocks[name] = IOBlock.new(name, direction)
    end
  end

  return self
end

-- Parse a CLB config from the bitstream format.
-- Converts the string-keyed table from JSON-like format.
function Fabric:_parse_clb_config(config)
  local result = {}
  if config.slice_0 then
    result.slice_0 = self:_parse_slice_config(config.slice_0)
  end
  if config.slice_1 then
    result.slice_1 = self:_parse_slice_config(config.slice_1)
  end
  return result
end

function Fabric:_parse_slice_config(config)
  local result = {}
  if config.lut_a then result.lut_a = config.lut_a end
  if config.lut_b then result.lut_b = config.lut_b end
  return result
end

--- Sets an input pin value on the fabric.
-- The pin must exist and must be an input or bidirectional I/O block.
--
-- @param pin_name  string name of the pin
-- @param value     0 or 1
-- @return self (for chaining)
function Fabric:set_input(pin_name, value)
  local io = self.io_blocks[pin_name]
  assert(io, "unknown pin: " .. tostring(pin_name))
  io:set_pin(value)
  return self
end

--- Reads an output pin value from the fabric.
--
-- @param pin_name  string name of the pin
-- @return  0, 1, or nil
function Fabric:read_output(pin_name)
  local io = self.io_blocks[pin_name]
  assert(io, "unknown pin: " .. tostring(pin_name))
  return io:read_pin()
end

--- Evaluates one clock cycle of the FPGA fabric.
-- Performs a simplified single-pass evaluation over all CLBs.
-- Returns self for chaining.
--
-- Note: This is a simplified model. Real FPGAs evaluate combinationally
-- in a single clock cycle with full signal propagation. Our model does
-- a single pass with zero inputs, which is sufficient for testing the
-- CLB evaluation pipeline.
--
-- @param clock  clock signal (0 or 1)
-- @return self (for chaining)
function Fabric:evaluate(clock)
  local zero_inputs = {}
  for i = 1, self.lut_inputs do
    zero_inputs[i] = 0
  end

  for _, clb in pairs(self.clbs) do
    local inputs = {
      s0_a = zero_inputs,
      s0_b = zero_inputs,
      s1_a = zero_inputs,
      s1_b = zero_inputs,
    }
    clb:evaluate(inputs, clock, 0)
  end

  return self
end

--- Returns a summary of the fabric's resources.
--
-- @return table with resource counts
function Fabric:summary()
  local clb_count = 0
  for _ in pairs(self.clbs) do clb_count = clb_count + 1 end

  local sm_count = 0
  for _ in pairs(self.switch_matrices) do sm_count = sm_count + 1 end

  local io_count = 0
  for _ in pairs(self.io_blocks) do io_count = io_count + 1 end

  return {
    rows               = self.rows,
    cols               = self.cols,
    clb_count          = clb_count,
    lut_count          = clb_count * 4,
    ff_count           = clb_count * 4,
    switch_matrix_count = sm_count,
    io_block_count     = io_count,
    lut_inputs         = self.lut_inputs,
  }
end

return Fabric
