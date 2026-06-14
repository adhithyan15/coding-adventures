package = "coding-adventures-neural-graph-vm"
version = "0.1.0-1"
source = { url = "git://github.com/adhithyan15/coding-adventures" }
description = { summary = "Scalar bytecode compiler for neural graphs", license = "MIT" }
build = { type = "builtin", modules = { ["coding_adventures.neural_graph_vm"] = "src/coding_adventures/neural_graph_vm/init.lua" } }
