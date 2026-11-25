package = "coding-adventures-microbenchmark"
version = "0.1.0-1"
source = { url = "git://github.com/adhithyan15/coding-adventures.git" }
description = {
    summary = "Paired microbenchmark collection and strict report replay",
    detailed = [[Reusable paired benchmark API, exact rank analysis, native monotonic clock, and array workload CLI for Lua 5.4.]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-rank-statistics >= 0.1.0",
    "coding-adventures-json-parser >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.microbenchmark"] = "src/coding_adventures/microbenchmark/init.lua",
        ["coding_adventures.microbenchmark.clock"] = "native/clock.c",
    },
    install = {
        bin = { ["array-microbenchmark"] = "script/array-microbenchmark" },
    },
}
