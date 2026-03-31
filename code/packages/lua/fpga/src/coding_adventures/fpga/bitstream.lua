-- bitstream.lua — FPGA configuration bitstream
--
-- A bitstream is a data structure that configures an FPGA fabric:
--   - clbs:    table of CLB configurations keyed by "row,col"
--   - routing: table of switch matrix configurations keyed by "row,col"
--   - io:      table of I/O configurations keyed by pin name
--
-- Example:
--   local bs = Bitstream.from_map({
--     clbs = {
--       ["0,0"] = {
--         slice_0 = { lut_a = {0,0,0,1}, lut_b = {0,1,1,0} },
--         slice_1 = { lut_a = {1,1,1,0} },
--       }
--     },
--     routing = {
--       ["0,0"] = { ["out_0"] = "in_2" }
--     },
--     io = {
--       top_0 = { direction = "input" },
--       bottom_0 = { direction = "output" },
--     }
--   })
--   local cfg = bs:clb_config("0,0")
--   local rcfg = bs:routing_config("0,0")
--   local icfg = bs:io_config("top_0")

local Bitstream = {}
Bitstream.__index = Bitstream

function Bitstream.from_map(config)
    return setmetatable({
        _clbs    = config.clbs    or {},
        _routing = config.routing or {},
        _io      = config.io      or {},
    }, Bitstream)
end

-- Returns CLB configuration for the given "row,col" key, or nil.
function Bitstream:clb_config(key)
    return self._clbs[key]
end

-- Returns routing configuration for the given "row,col" key, or nil.
function Bitstream:routing_config(key)
    return self._routing[key]
end

-- Returns I/O configuration for the given pin name, or nil.
function Bitstream:io_config(pin_name)
    return self._io[pin_name]
end

return Bitstream
