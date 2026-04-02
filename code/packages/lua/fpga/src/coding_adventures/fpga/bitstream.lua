--[[
  Bitstream — FPGA configuration data.

  ## What is a Bitstream?

  A bitstream is the configuration file that programs an FPGA. It contains
  all the information needed to configure every LUT, flip-flop, routing
  switch, I/O block, and Block RAM in the device. When you "synthesize"
  and "place & route" a hardware design (written in VHDL or Verilog), the
  tool chain produces a bitstream file.

  In real FPGAs, bitstreams are binary files with vendor-specific formats
  (e.g., Xilinx .bit files, Intel .sof files). They are typically loaded
  into the FPGA at power-up from an external flash memory chip.

  ## Our Format

  For this educational implementation, we use plain Lua tables as our
  "bitstream" format. The table has this structure:

      {
        clbs = {
          ["0_0"] = {
            slice_0 = {
              lut_a = {0, 0, 0, 1, ...},
              lut_b = {0, 1, 1, 0, ...},
              use_ff_a     = false,
              use_ff_b     = true,
              carry_enable = false,
            },
            slice_1 = { ... },
          },
          ...
        },
        routing = {
          ["0_0"] = { out_0 = "in_2", ... },
          ...
        },
        io = {
          pin_0 = { direction = "input" },
          pin_1 = { direction = "output" },
          ...
        }
      }

  This table-based format is easy to construct and inspect, making it
  ideal for learning and testing.
]]

local Bitstream = {}
Bitstream.__index = Bitstream

--- Parses a bitstream from a plain Lua table.
-- The table should have top-level keys "clbs", "routing", and "io".
-- Missing keys default to empty tables.
--
-- @param config  Lua table with clbs/routing/io structure
-- @return new Bitstream object
function Bitstream.from_map(config)
  assert(type(config) == "table", "config must be a table")

  return setmetatable({
    clb_configs     = config.clbs    or {},
    routing_configs = config.routing or {},
    io_configs      = config.io      or {},
  }, Bitstream)
end

--- Returns the CLB configuration for the given position key (e.g., "0_0").
-- Returns nil if no configuration exists for that position.
--
-- @param key  string like "row_col"
-- @return  configuration table or nil
function Bitstream:clb_config(key)
  return self.clb_configs[key]
end

--- Returns the routing configuration for the given position key.
--
-- @param key  string like "row_col"
-- @return  routing connections table or nil
function Bitstream:routing_config(key)
  return self.routing_configs[key]
end

--- Returns the I/O configuration for the given pin name.
--
-- @param pin_name  string pin identifier
-- @return  configuration table or nil
function Bitstream:io_config(pin_name)
  return self.io_configs[pin_name]
end

return Bitstream
