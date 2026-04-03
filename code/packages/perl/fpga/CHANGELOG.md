# Changelog — CodingAdventures::FPGA (Perl)

## 0.1.0 — 2026-03-31

### Added
- Complete FPGA simulation library in Pure Perl (5.26+)
- **LUT** (Lookup Table): N-input truth table; MSB-first address indexing;
  `new($n)`, `configure(\@tt)`, `evaluate(\@inputs)` → 0 or 1
- **Slice**: 2 LUTs + 2 D flip-flops (rising-edge) + carry chain;
  `new(%opts)` (lut_inputs, use_ff_a, use_ff_b, carry_enable),
  `configure(\%config)`, `evaluate(\@a, \@b, $clock, $carry_in)` → (out_a, out_b, carry_out)
- **CLB**: 2 slices with carry propagation Slice0→Slice1;
  `new($row, $col, %opts)`, `configure(\%config)`,
  `evaluate(\%inputs, $clock, $carry_in)` → (\@outputs, carry_out)
- **SwitchMatrix**: programmable routing crossbar with output→input connections;
  `new($n_in, $n_out)`, `configure(\%connections)`, `route(\%signals)` → \%result
- **IOBlock**: external pin interface in input/output/bidirectional modes;
  `new($name, $dir)`, `set_pin()`, `set_fabric()`, `set_output_enable()`,
  `read_fabric()`, `read_pin()`
- **Fabric**: complete FPGA; rows×cols CLB grid; perimeter I/O blocks;
  switch matrix per CLB; `new($rows, $cols, %opts)`, `load_bitstream()`,
  `set_input()`, `read_output()`, `evaluate()`, `summary()`
- **Bitstream**: configuration parser from Perl hashrefs;
  `from_map(\%config)`, `clb_config($key)`, `routing_config($key)`, `io_config($pin)`
- Comprehensive test suite: unit tests for all 7 components + end-to-end
  AND gate programmed onto FPGA fabric
