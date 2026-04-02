--[[
  coding_adventures.fpga — FPGA (Field-Programmable Gate Array) simulation.

  ## Overview

  An FPGA is a chip full of logic gates, memory, and wires — but unlike a
  CPU or GPU where the circuits are permanently etched in silicon, an FPGA's
  circuits are **programmable**. You upload a configuration file called a
  **bitstream** and the chip reconfigures itself to implement whatever digital
  circuit you described.

  This package models the key FPGA components:

    LUT          — Lookup Table: stores a truth table, implements any Boolean function
    Slice        — 2 LUTs + 2 flip-flops + carry chain
    CLB          — Configurable Logic Block: 2 slices (4 LUTs, 4 FFs, 2 carry chains)
    SwitchMatrix — Programmable routing crossbar
    IOBlock      — I/O interface between fabric and external pins
    Fabric       — Complete FPGA: CLB grid + routing + I/O
    Bitstream    — Configuration data parser

  ## The Big Idea

  A truth table is a program.

  A 4-input LUT loaded with {0,0,0,0,0,0,0,1,...} is an AND gate.
  Load it with {0,1,1,0,1,0,0,1,...} and it becomes an XOR gate.
  Same silicon, different function. This is what makes FPGAs "programmable".

  ## Quick Start

      local fpga = require("coding_adventures.fpga")

      -- Create a 2x2 FPGA fabric with 2-input LUTs
      local fabric = fpga.Fabric.new(2, 2, { lut_inputs = 2 })

      -- Program it with a bitstream (AND gate on CLB 0_0)
      local bs = fpga.Bitstream.from_map({
        clbs = {
          ["0_0"] = {
            slice_0 = {
              lut_a = {0, 0, 0, 1},   -- AND truth table
            }
          }
        },
        routing = {},
        io = {},
      })
      fabric:load_bitstream(bs)

      -- Evaluate
      fabric:evaluate(1)

      -- Inspect
      local s = fabric:summary()
      print("CLBs: " .. s.clb_count)  -- 4
      print("LUTs: " .. s.lut_count)  -- 16

  ## Package Structure

      coding_adventures.fpga           — this file (top-level re-export)
      coding_adventures.fpga.lut       — LUT implementation
      coding_adventures.fpga.slice     — Slice (2 LUTs + 2 FFs + carry)
      coding_adventures.fpga.clb       — CLB (2 slices)
      coding_adventures.fpga.switch_matrix — Routing crossbar
      coding_adventures.fpga.io_block  — I/O interface
      coding_adventures.fpga.fabric    — Complete FPGA top-level
      coding_adventures.fpga.bitstream — Configuration parser
]]

local M = {}

M.LUT          = require("coding_adventures.fpga.lut")
M.Slice        = require("coding_adventures.fpga.slice")
M.CLB          = require("coding_adventures.fpga.clb")
M.SwitchMatrix = require("coding_adventures.fpga.switch_matrix")
M.IOBlock      = require("coding_adventures.fpga.io_block")
M.Fabric       = require("coding_adventures.fpga.fabric")
M.Bitstream    = require("coding_adventures.fpga.bitstream")

return M
