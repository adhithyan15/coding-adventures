package = "coding-adventures-rank-statistics"
version = "0.1.0-1"
source = { url = "git://github.com/adhithyan15/coding-adventures.git" }
description = {
    summary = "Finite-sample medians and exact median confidence intervals",
    detailed = [[Reusable non-mutating sample median and exact order-statistic interval operations for Lua 5.4.]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}
dependencies = { "lua >= 5.4" }
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.rank_statistics"] = "src/coding_adventures/rank_statistics/init.lua",
    },
}
