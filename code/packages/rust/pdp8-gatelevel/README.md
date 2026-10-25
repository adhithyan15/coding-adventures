# DEC PDP-8/E gate-level simulator

This crate is the digital-logic partner for `pdp8-simulator`. All 49,205
architectural state bits—4K words of core memory, AC, PC, MQ, switch register,
link, interrupt controls, and halt—are held in repository D flip-flops.

Opcode and OPR selection use Boolean gates. Address and instruction selection
use multiplexers; TAD, ISZ, IAC, auto-index, JMS, skips, interrupt entry, and PC
movement use ripple-carry networks; AND and skip predicates use Boolean gates;
and rotate/swap paths are explicit wires. Host control sequences clock edges,
selects a completed 12-bit address, formats traces, and carries lifecycle
metadata. It does not calculate architectural data results.

The model is an educational gate topology corresponding to the ISA-visible
PDP-8/E processor, not a cycle-accurate reconstruction of DEC's M8300-series
boards or a transistor/layout model. Its source boundary and supported
optional-hardware exclusions are specified in
[`../../../specs/07aa2-pdp8-gatelevel.md`](../../../specs/07aa2-pdp8-gatelevel.md).

## Development

```bash
bash BUILD
```
