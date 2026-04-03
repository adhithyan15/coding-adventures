package = "coding-adventures-fpga"
version = "0.1.0-1"

source = {
  url = "https://github.com/adhithyan15/coding-adventures",
}

description = {
  summary  = "FPGA (Field-Programmable Gate Array) simulation",
  detailed = [[
    Models the key FPGA components: LUT (truth-table lookup), Slice (2 LUTs +
    2 flip-flops + carry chain), CLB (2 slices), SwitchMatrix (programmable
    routing crossbar), IOBlock (external pin interface), Fabric (complete
    FPGA top-level), and Bitstream (configuration data parser).
  ]],
  license  = "MIT",
}

dependencies = {
  "lua >= 5.4",
  "coding-adventures-logic-gates >= 0.1.0",
  "coding-adventures-block-ram >= 0.1.0",
}

build = {
  type = "builtin",
  modules = {
    ["coding_adventures.fpga"]               = "src/coding_adventures/fpga/init.lua",
    ["coding_adventures.fpga.lut"]           = "src/coding_adventures/fpga/lut.lua",
    ["coding_adventures.fpga.slice"]         = "src/coding_adventures/fpga/slice.lua",
    ["coding_adventures.fpga.clb"]           = "src/coding_adventures/fpga/clb.lua",
    ["coding_adventures.fpga.switch_matrix"] = "src/coding_adventures/fpga/switch_matrix.lua",
    ["coding_adventures.fpga.io_block"]      = "src/coding_adventures/fpga/io_block.lua",
    ["coding_adventures.fpga.fabric"]        = "src/coding_adventures/fpga/fabric.lua",
    ["coding_adventures.fpga.bitstream"]     = "src/coding_adventures/fpga/bitstream.lua",
  },
}
