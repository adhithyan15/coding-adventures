package = "coding-adventures-cpu-pipeline"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Configurable N-stage CPU instruction pipeline",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.cpu_pipeline"]          = "src/coding_adventures/cpu_pipeline/init.lua",
        ["coding_adventures.cpu_pipeline.token"]    = "src/coding_adventures/cpu_pipeline/token.lua",
        ["coding_adventures.cpu_pipeline.config"]   = "src/coding_adventures/cpu_pipeline/config.lua",
        ["coding_adventures.cpu_pipeline.pipeline"] = "src/coding_adventures/cpu_pipeline/pipeline.lua",
    },
}
