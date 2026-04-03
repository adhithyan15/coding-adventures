package = "coding-adventures-clr-simulator"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "CLR (Common Language Runtime) IL bytecode simulator",
    detailed = [[
        Simulates a subset of the CLR Intermediate Language (IL/CIL/MSIL).
        Supports ldc.i4 variants, ldloc/stloc, arithmetic (add/sub/mul/div),
        compare instructions (ceq/cgt/clt with 0xFE prefix), branching
        (br.s/brfalse.s/brtrue.s), and ret. Includes full step tracing.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.clr_simulator"] = "src/coding_adventures/clr_simulator/init.lua",
    },
}
