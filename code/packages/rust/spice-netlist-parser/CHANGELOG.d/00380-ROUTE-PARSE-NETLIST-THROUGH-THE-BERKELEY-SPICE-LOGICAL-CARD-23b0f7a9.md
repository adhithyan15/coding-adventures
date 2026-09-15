- Route `parse_netlist` through the Berkeley SPICE logical-card syntax facade,
  so the default Rust parser consumes normalized cards, supports leading `+`
  continuations, and reports stable syntax diagnostics before semantic
  lowering.
