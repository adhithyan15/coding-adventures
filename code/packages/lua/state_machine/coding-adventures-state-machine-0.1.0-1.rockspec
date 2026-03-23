package = "coding-adventures-state-machine"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Finite automata (DFA, NFA, PDA, Modal, Minimize) built on directed graphs",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-directed-graph >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.state_machine"] = "src/coding_adventures/state_machine/init.lua",
    },
}
