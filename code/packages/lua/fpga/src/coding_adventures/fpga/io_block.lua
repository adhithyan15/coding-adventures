-- io_block.lua — I/O Block: interface between FPGA fabric and external pins
--
-- An IOBlock sits at the FPGA perimeter. It can be:
--   "input"        — reads external pin into fabric
--   "output"       — drives pin from fabric
--   "bidirectional"— can do both (output_enable controls direction)
--
-- For input blocks:  set_pin(v) then read_fabric()
-- For output blocks: set_fabric(v) then read_pin()
-- For bidirectional: use set_output_enable() to switch direction

local IOBlock = {}
IOBlock.__index = IOBlock

function IOBlock.new(name, direction)
    assert(direction == "input" or direction == "output" or direction == "bidirectional",
        "direction must be 'input', 'output', or 'bidirectional'")
    return setmetatable({
        name           = name,
        direction      = direction,
        pin_value      = nil,
        fabric_value   = nil,
        output_enable  = (direction == "output") and 1 or 0,
    }, IOBlock)
end

-- Sets the external pin value (for input/bidirectional blocks).
function IOBlock:set_pin(v)
    assert(self.direction ~= "output",
        "cannot set pin on output-only I/O block")
    self.pin_value = v
end

-- Sets the fabric value (for output/bidirectional blocks).
function IOBlock:set_fabric(v)
    assert(self.direction ~= "input",
        "cannot set fabric on input-only I/O block")
    self.fabric_value = v
end

-- Sets output enable (for bidirectional blocks). 1=output, 0=input.
function IOBlock:set_output_enable(v)
    self.output_enable = v
end

-- Reads the fabric-facing signal.
-- For input blocks: returns pin_value (external → fabric)
-- For bidirectional with output_enable=0: returns pin_value
function IOBlock:read_fabric()
    if self.direction == "output" then
        return self.fabric_value
    end
    if self.direction == "bidirectional" then
        if self.output_enable == 0 then
            return self.pin_value
        else
            return self.fabric_value
        end
    end
    return self.pin_value
end

-- Reads the external pin.
-- For output blocks: returns fabric_value (fabric → pin)
-- For bidirectional with output_enable=1: returns fabric_value
function IOBlock:read_pin()
    if self.direction == "input" then
        return self.pin_value
    end
    if self.direction == "bidirectional" then
        if self.output_enable == 1 then
            return self.fabric_value
        else
            return self.pin_value
        end
    end
    return self.fabric_value
end

return IOBlock
