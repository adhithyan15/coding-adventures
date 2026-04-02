package = "coding-adventures-arm1-simulator"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "ARM1 (ARMv1) behavioral instruction set simulator — complete ARMv1 ISA with barrel shifter, ALU, and encoding helpers",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.arm1_simulator"] = "src/coding_adventures/arm1_simulator/init.lua",
    },
}
