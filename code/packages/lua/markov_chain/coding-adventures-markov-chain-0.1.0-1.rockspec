package = "coding-adventures-markov-chain"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "General-purpose Markov Chain (DT28)",
    detailed = [[
        A general-purpose Markov Chain library implementing order-k chains,
        Laplace/Lidstone smoothing, sequence generation, and stationary
        distribution computation via power iteration. Built on top of the
        coding-adventures directed graph package for topology tracking.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-directed-graph",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.markov_chain"] = "src/coding_adventures/markov_chain/init.lua",
    },
}
