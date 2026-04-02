-- switch_matrix.lua — Programmable routing crossbar in an FPGA
--
-- A SwitchMatrix connects N input ports to M output ports via a programmable
-- routing map. Each output can connect to at most one input. Multiple outputs
-- can share the same input (fan-out is allowed).
--
-- Port names: "in_0".."in_{N-1}", "out_0".."out_{M-1}"
--
-- route(input_signals) takes a table of {port_name → value} and returns a
-- table of {output_port_name → value}. Unconnected outputs are nil.

local SwitchMatrix = {}
SwitchMatrix.__index = SwitchMatrix

function SwitchMatrix.new(num_inputs, num_outputs)
    assert(num_inputs  > 0, "num_inputs must be > 0")
    assert(num_outputs > 0, "num_outputs must be > 0")

    local input_names  = {}
    local output_names = {}
    for i = 0, num_inputs  - 1 do table.insert(input_names,  "in_"  .. i) end
    for i = 0, num_outputs - 1 do table.insert(output_names, "out_" .. i) end

    return setmetatable({
        num_inputs   = num_inputs,
        num_outputs  = num_outputs,
        connections  = {},  -- {out_name → in_name}
        input_names  = input_names,
        output_names = output_names,
    }, SwitchMatrix)
end

-- Configures routing. connections is a table {out_name → in_name}.
-- Validates port names. Fan-out (multiple outputs to same input) is allowed.
function SwitchMatrix:configure(connections)
    -- Build sets for fast lookup
    local in_set, out_set = {}, {}
    for _, n in ipairs(self.input_names)  do in_set[n]  = true end
    for _, n in ipairs(self.output_names) do out_set[n] = true end

    for out_name, in_name in pairs(connections) do
        assert(out_set[out_name],
            "invalid output port: " .. tostring(out_name))
        assert(in_set[in_name],
            "invalid input port: " .. tostring(in_name))
    end
    self.connections = connections
    return self
end

-- Routes signals through the matrix.
-- input_signals: table {in_name → value}
-- Returns: table {out_name → value} (nil for unconnected outputs)
function SwitchMatrix:route(input_signals)
    local output_signals = {}
    for _, out_name in ipairs(self.output_names) do
        local in_name = self.connections[out_name]
        if in_name then
            output_signals[out_name] = input_signals[in_name]
        else
            output_signals[out_name] = nil
        end
    end
    return output_signals
end

return SwitchMatrix
