package = "coding-adventures-neural-network"
version = "0.1.0-1"
source = { url = "git://github.com/adhithyan15/coding-adventures" }
description = { summary = "Graph-native neural network primitives", license = "MIT" }
build = { type = "builtin", modules = { ["coding_adventures.neural_network"] = "src/coding_adventures/neural_network/init.lua" } }
