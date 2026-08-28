# DEC PDP-11 gate-level simulator

This Rust crate is the gate-level companion to `pdp11-simulator`. It preserves
the complete Spec 07o functional surface while representing all 524,433
persistent architectural bits as simulated master-slave D flip-flops.

Instruction recognition uses gate comparators. Architectural arithmetic,
logic, NZVC flags, effective-address side effects, branch offsets, stack
updates, byte sign extension, and PC movement flow through gate vectors. Host
integers are limited to checked memory/register selection, sequencing,
transport conversion, and owned observations.

See [Spec 07o2](../../../specs/07o2-pdp11-gatelevel.md).
