-- Rockspec for coding-adventures-gpu-core
-- =========================================
--
-- This rockspec declares the GPU core as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/gpu_core/
--
-- The gpu_core package has no sibling dependencies — it is a self-contained
-- simulation of a generic accelerator processing element.

package = "coding-adventures-gpu-core"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "Generic accelerator processing element — GPU core simulator",
    detailed = [[
        A generic, pluggable accelerator processing element that models the
        smallest compute unit in any accelerator: CUDA Core, Stream Processor,
        Vector Engine, or MAC Unit. Includes an FP register file, local
        scratchpad memory, a Generic ISA with FP arithmetic/memory/branch
        instructions, and a fetch-execute loop with cycle-accurate tracing.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.gpu_core"] =
            "src/coding_adventures/gpu_core/init.lua",
    },
}
