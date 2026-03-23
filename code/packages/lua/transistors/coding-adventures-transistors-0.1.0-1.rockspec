package = "coding-adventures-transistors"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "MOSFET, BJT, CMOS, and TTL transistor-level circuit simulation",
    detailed = [[
        Transistor-level circuit simulation covering NMOS/PMOS MOSFETs,
        NPN/PNP BJTs, CMOS logic gates (NOT, NAND, NOR, AND, OR, XOR),
        TTL NAND, RTL inverter, analog amplifier analysis, noise margins,
        power analysis, timing analysis, and CMOS scaling demonstration.
        Ported from the Go implementation in the coding-adventures monorepo.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.transistors"]            = "src/coding_adventures/transistors/init.lua",
        ["coding_adventures.transistors.types"]       = "src/coding_adventures/transistors/types.lua",
        ["coding_adventures.transistors.mosfet"]      = "src/coding_adventures/transistors/mosfet.lua",
        ["coding_adventures.transistors.bjt"]         = "src/coding_adventures/transistors/bjt.lua",
        ["coding_adventures.transistors.cmos_gates"]  = "src/coding_adventures/transistors/cmos_gates.lua",
        ["coding_adventures.transistors.ttl_gates"]   = "src/coding_adventures/transistors/ttl_gates.lua",
        ["coding_adventures.transistors.amplifier"]   = "src/coding_adventures/transistors/amplifier.lua",
        ["coding_adventures.transistors.analysis"]    = "src/coding_adventures/transistors/analysis.lua",
    },
}
