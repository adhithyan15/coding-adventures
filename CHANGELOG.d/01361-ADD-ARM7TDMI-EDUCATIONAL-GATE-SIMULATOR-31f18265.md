# Added

- Added the educational ARM7TDMI gate-level companion with an exact
  525,522-DFF stable-Q inventory, independent ARM/Thumb transition engine,
  gate-derived condition/arithmetic/control networks, full-state differential
  verification, typed transactional lifecycle, and structural anti-bypass
  checks. Parsed exact locks cover every specified architectural route, and
  compiler-valid hostile mutations reject host arithmetic/decode, oracle
  substitution, dead anchors, direct storage writes, and post-gate/commit
  overwrites. Two-way candidate divergence and every typed fault/completion
  class confirm stable-Q authority. This is an ISA-level teaching topology,
  not Arm's silicon netlist.
