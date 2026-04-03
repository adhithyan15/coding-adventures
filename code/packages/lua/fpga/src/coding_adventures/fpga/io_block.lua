--[[
  IOBlock — Input/Output interface between the FPGA fabric and external pins.

  ## What is an I/O Block?

  I/O blocks sit at the perimeter of the FPGA die and provide the interface
  between the internal logic fabric and the external package pins. Each I/O
  block can be configured as:

    - "input"       — reads an external signal into the FPGA
    - "output"      — drives an external pin from the FPGA
    - "bidirectional" — can switch between input and output (tri-state)

  ## I/O Block Components

      External Pin
          │
      ┌───┴───┐
      │ Pad   │ ← Physical connection to package pin
      ├───────┤
      │ Input │ ← Input buffer (signal from pin into fabric)
      │ Buffer│
      ├───────┤
      │Output │ ← Output buffer (signal from fabric to pin)
      │Buffer │
      ├───────┤
      │Tri-St │ ← Output enable (for bidirectional I/O)
      │Control│
      └───────┘
          │
      To/From Internal Fabric

  ## Signal Flow

  For an input block:   external_pin → pin_value → read_fabric()
  For an output block:  set_fabric() → fabric_value → read_pin()
  For bidirectional:    OE=1: set_fabric → pin; OE=0: pin → read_fabric
]]

local IOBlock = {}
IOBlock.__index = IOBlock

local VALID_DIRECTIONS = { input = true, output = true, bidirectional = true }

--- Creates a new I/O block with the given name and direction.
--
-- @param name       string identifier for this I/O pin
-- @param direction  "input", "output", or "bidirectional"
-- @return new IOBlock object
function IOBlock.new(name, direction)
  assert(VALID_DIRECTIONS[direction],
    'direction must be "input", "output", or "bidirectional", got: ' .. tostring(direction))

  return setmetatable({
    name           = name,
    direction      = direction,
    pin_value      = nil,
    fabric_value   = nil,
    -- Output blocks are always enabled; others default to disabled
    output_enable  = (direction == "output") and 1 or 0,
  }, IOBlock)
end

--- Sets the external pin value (used for input and bidirectional blocks).
-- Simulates an external device driving a signal onto the pin.
--
-- @param value  0 or 1
-- @return self (for chaining)
function IOBlock:set_pin(value)
  assert(self.direction ~= "output",
    "cannot set pin on output-only I/O block")
  assert(value == 0 or value == 1,
    "pin value must be 0 or 1, got: " .. tostring(value))

  self.pin_value = value
  return self
end

--- Sets the fabric-side value (used for output and bidirectional blocks).
-- This is the value that the internal FPGA logic wants to drive onto
-- the external pin.
--
-- @param value  0 or 1
-- @return self (for chaining)
function IOBlock:set_fabric(value)
  assert(self.direction ~= "input",
    "cannot set fabric value on input-only I/O block")
  assert(value == 0 or value == 1,
    "fabric value must be 0 or 1, got: " .. tostring(value))

  self.fabric_value = value
  return self
end

--- Sets the output enable signal (for bidirectional blocks only).
-- OE=1: block drives pin from fabric_value.
-- OE=0: block is in high-Z state (acts as input).
--
-- @param value  0 or 1
-- @return self (for chaining)
function IOBlock:set_output_enable(value)
  assert(self.direction == "bidirectional",
    "output enable only applies to bidirectional I/O, got: " .. self.direction)
  assert(value == 0 or value == 1,
    "output_enable must be 0 or 1, got: " .. tostring(value))

  self.output_enable = value
  return self
end

--- Reads the value available to the internal fabric.
--
-- input block:       returns pin_value
-- output block:      returns fabric_value
-- bidirectional OE=0: returns pin_value (reading from external)
-- bidirectional OE=1: returns fabric_value (driving out, loop-back)
--
-- @return  0, 1, or nil
function IOBlock:read_fabric()
  if self.direction == "input" then
    return self.pin_value
  elseif self.direction == "output" then
    return self.fabric_value
  else  -- bidirectional
    if self.output_enable == 0 then
      return self.pin_value
    else
      return self.fabric_value
    end
  end
end

--- Reads the value on the external pin.
--
-- input block:         returns pin_value (driven by external)
-- output block:        returns fabric_value (driven by FPGA)
-- bidirectional OE=1:  returns fabric_value (FPGA is driving)
-- bidirectional OE=0:  returns pin_value (external is driving)
--
-- @return  0, 1, or nil
function IOBlock:read_pin()
  if self.direction == "input" then
    return self.pin_value
  elseif self.direction == "output" then
    return self.fabric_value
  else  -- bidirectional
    if self.output_enable == 1 then
      return self.fabric_value
    else
      return self.pin_value
    end
  end
end

return IOBlock
