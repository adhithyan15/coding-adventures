package = "coding-adventures-arm1-gatelevel"
version = "0.1.0-1"

source = {
  url = "https://github.com/adhithyan15/coding-adventures",
}

description = {
  summary  = "ARM1 gate-level processor simulator",
  detailed = [[
    Models the first ARM processor at the hardware level. Every arithmetic
    operation routes through actual logic gate function calls (AND, OR, XOR,
    NOT) chained into adders, then into a 32-bit ALU. The barrel shifter is
    built from a 5-level tree of Mux2 gates. Registers are stored as bit
    arrays (LSB-first).
  ]],
  license  = "MIT",
}

dependencies = {
  "lua >= 5.4",
  "coding-adventures-logic-gates",
  "coding-adventures-arithmetic",
  "coding-adventures-arm1-simulator",
}

build = {
  type = "builtin",
  modules = {
    ["coding_adventures.arm1_gatelevel"] = "src/coding_adventures/arm1_gatelevel/init.lua",
  },
}
