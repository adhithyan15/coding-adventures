# Changelog

All notable changes to the intel4004_gatelevel package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-21

### Added
- Complete Intel 4004 gate-level simulator with all 46 instructions + HLT
- All operations route through logic gates, adders, ALU, and D flip-flops
- Two-phase clock write cycle (clock=0 captures master, clock=1 latches slave)
- Bit helpers for integer ↔ LSB-first bit list conversion
- Gate-level instruction decoder using AND/OR/NOT gate trees
- PC increment via half-adder chain
- JCN condition evaluation through gate-level OR/AND/NOT
- ISZ zero-detection through gate-level NOR
- RAM stored as map of flip-flop states for each nibble
- Execution tracing with before/after accumulator and carry state
- Gate count estimation (~8894 gates)
- 56 tests covering all instructions and end-to-end programs
