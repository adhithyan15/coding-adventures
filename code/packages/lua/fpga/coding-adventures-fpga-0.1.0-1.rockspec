package = "coding-adventures-fpga"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "FPGA fabric simulation — LUTs, slices, CLBs, switch matrices, I/O blocks, and bitstream loading",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
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
        ["coding_adventures.fpga.bitstream"]     = "src/coding_adventures/fpga/bitstream.lua",
        ["coding_adventures.fpga.fabric"]        = "src/coding_adventures/fpga/fabric.lua",
    },
}
