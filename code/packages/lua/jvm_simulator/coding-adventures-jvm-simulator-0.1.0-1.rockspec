package = "coding-adventures-jvm-simulator"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "JVM (Java Virtual Machine) bytecode simulator",
    detailed = [[
        Simulates a subset of JVM bytecode. Supports iconst_0-5, bipush,
        sipush, ldc, iload/istore (short and long forms), iadd/isub/imul/idiv,
        goto, if_icmpeq/if_icmpgt, ireturn, and return.
        Full step-level tracing with before/after stack and locals snapshots.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.jvm_simulator"] = "src/coding_adventures/jvm_simulator/init.lua",
    },
}
