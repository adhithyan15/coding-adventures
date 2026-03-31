-- coding_adventures.fpga — FPGA fabric simulation
--
-- Exports all FPGA components:
--   LUT          — Lookup Table (truth-table programmable logic)
--   Slice        — 2 LUTs + 2 FFs + carry chain
--   CLB          — 2 Slices with carry propagation
--   SwitchMatrix — Programmable routing crossbar
--   IOBlock      — I/O interface to external pins
--   Bitstream    — Configuration data structure
--   Fabric       — Full rows×cols CLB grid

return {
    VERSION      = "0.1.0",
    LUT          = require("coding_adventures.fpga.lut"),
    Slice        = require("coding_adventures.fpga.slice"),
    CLB          = require("coding_adventures.fpga.clb"),
    SwitchMatrix = require("coding_adventures.fpga.switch_matrix"),
    IOBlock      = require("coding_adventures.fpga.io_block"),
    Bitstream    = require("coding_adventures.fpga.bitstream"),
    Fabric       = require("coding_adventures.fpga.fabric"),
}
