--[[
  SwitchMatrix — programmable routing crossbar in an FPGA.

  ## What is a Switch Matrix?

  A switch matrix (also called a routing switch box) is the component that
  connects CLBs, I/O blocks, and other resources together. It sits at the
  intersection of horizontal and vertical routing channels.

  In a real FPGA, the routing network consumes 60–80% of the chip area
  and contributes significantly to signal delay. The switch matrix contains
  programmable pass transistors that can be turned on or off to create
  connections between wires.

  ## Switch Matrix Architecture

      Input 0 ──┐
      Input 1 ──┤──── Switch ──── Output 0
      Input 2 ──┤     Matrix ──── Output 1
      Input 3 ──┘            ──── Output 2

  Each output port can be connected to at most one input port
  (to avoid driver conflicts). Multiple outputs may share the same input
  (fan-out is allowed).

  ## Configuration

  The routing map is a table from output port names to input port names:

      { out_0 = "in_2", out_1 = "in_0" }

  This means output 0 is connected to input 2, and output 1 is
  connected to input 0. Unconnected outputs produce nil (high-Z).

  ## Why This Matters

  The switch matrix is what makes an FPGA "field-programmable" — by
  changing the routing configuration, you change how logic blocks are
  connected, which changes the overall circuit behavior.
]]

local SwitchMatrix = {}
SwitchMatrix.__index = SwitchMatrix

--- Creates a new switch matrix.
-- Port names are automatically generated as "in_0", "in_1", ...
-- and "out_0", "out_1", ...
--
-- @param num_inputs   number of input ports (positive integer)
-- @param num_outputs  number of output ports (positive integer)
-- @return new SwitchMatrix object
function SwitchMatrix.new(num_inputs, num_outputs)
  assert(type(num_inputs) == "number" and num_inputs > 0,
    "num_inputs must be a positive integer")
  assert(type(num_outputs) == "number" and num_outputs > 0,
    "num_outputs must be a positive integer")

  local input_names  = {}
  local output_names = {}
  local input_set    = {}
  local output_set   = {}

  for i = 0, num_inputs - 1 do
    local name = "in_" .. i
    input_names[i + 1]  = name
    input_set[name]      = true
  end

  for i = 0, num_outputs - 1 do
    local name = "out_" .. i
    output_names[i + 1] = name
    output_set[name]     = true
  end

  return setmetatable({
    num_inputs   = num_inputs,
    num_outputs  = num_outputs,
    connections  = {},
    input_names  = input_names,
    output_names = output_names,
    _input_set   = input_set,
    _output_set  = output_set,
  }, SwitchMatrix)
end

--- Configures the switch matrix routing.
-- connections is a table from output port names to input port names.
-- Raises an error if any port name is invalid.
--
-- @param connections  table: { out_name = in_name, ... }
-- @return self (for chaining)
function SwitchMatrix:configure(connections)
  for out_name, in_name in pairs(connections) do
    assert(self._output_set[out_name],
      "invalid output port: " .. tostring(out_name))
    assert(self._input_set[in_name],
      "invalid input port: " .. tostring(in_name))
  end

  self.connections = connections
  return self
end

--- Routes signals through the switch matrix.
-- input_signals is a table of input port names to signal values (0, 1, or nil).
-- Returns a table of output port names to signal values.
-- Unconnected outputs have nil values (high-Z / undriven).
--
-- @param input_signals  table: { in_name = value, ... }
-- @return table: { out_name = value_or_nil, ... }
function SwitchMatrix:route(input_signals)
  local result = {}

  for _, out_name in ipairs(self.output_names) do
    local in_name = self.connections[out_name]
    if in_name then
      result[out_name] = input_signals[in_name]
    else
      -- Unconnected — high-Z
      result[out_name] = nil
    end
  end

  return result
end

return SwitchMatrix
