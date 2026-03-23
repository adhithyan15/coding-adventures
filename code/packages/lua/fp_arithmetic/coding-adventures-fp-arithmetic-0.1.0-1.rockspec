package = "coding-adventures-fp-arithmetic"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "IEEE 754 floating-point arithmetic from logic gates -- FP32, FP16, BF16 formats",
    detailed = [[
        Implements the complete IEEE 754 floating-point arithmetic stack:
        encoding/decoding, addition, subtraction, multiplication, fused
        multiply-add, format conversion, and pipelined hardware simulation.
        All built on top of logic gates, just like real hardware.

        Supports FP32 (single precision), FP16 (half precision), and
        BF16 (brain float) formats used in CPUs, GPUs, and TPUs.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-clock >= 0.1.0",
    "coding-adventures-logic-gates >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.fp_arithmetic"] = "src/coding_adventures/fp_arithmetic/init.lua",
        ["coding_adventures.fp_arithmetic.formats"] = "src/coding_adventures/fp_arithmetic/formats.lua",
        ["coding_adventures.fp_arithmetic.ieee754"] = "src/coding_adventures/fp_arithmetic/ieee754.lua",
        ["coding_adventures.fp_arithmetic.fp_adder"] = "src/coding_adventures/fp_arithmetic/fp_adder.lua",
        ["coding_adventures.fp_arithmetic.fp_multiplier"] = "src/coding_adventures/fp_arithmetic/fp_multiplier.lua",
        ["coding_adventures.fp_arithmetic.fma"] = "src/coding_adventures/fp_arithmetic/fma.lua",
        ["coding_adventures.fp_arithmetic.pipeline"] = "src/coding_adventures/fp_arithmetic/pipeline.lua",
    },
}
