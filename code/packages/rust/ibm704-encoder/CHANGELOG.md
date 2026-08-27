# Changelog — ibm704-encoder

## v0.2.0 — 2026-08-27 — canonical IBM 704 words

* Replaced the idealized word layout with the historical Type A and Type B
  fields from the 1955 IBM manual.
* Corrected HTR to `+0000`; added the distinct HPR `+0420` convenience.
* Changed the producer transport to the simulator's five-byte big-endian
  contract and added strict decoding helpers.
* Rejects prefixes that cannot architecturally identify Type A words through
  a non-panicking typed encoder error.
* Added field-boundary, signed-operation, invalid-transport, and round-trip
  tests.

## v0.1.0 — 2026-06-11 — initial release (L4)

Phase L4 of the McCarthy Lisp implementation.

* Opcode constants: `HTR` (0o420, Halt and Transfer — used as
  halt sentinel via `HTR 0` per the GE-225 / Intel 4004 idiom)
  and `CLA` (0o500, Clear and Add Accumulator — used to load a
  15-bit immediate into the accumulator).
* `encode_htr(addr)` / `encode_cla(addr)` build a 36-bit word
  with the opcode in bits 35..27 and a 15-bit address in bits
  14..0.
* `pack_word(word)` writes a 36-bit word as 5 bytes, low byte
  first, high 4 bits zeroed — the same packing convention
  GE-225 uses (3 bytes for 20 bits) extended to 5 bytes for 36
  bits.
* `HTR_HALT_BYTES` — pre-computed packing of `HTR 0`
  (`[0x00, 0x00, 0x00, 0x80, 0x08]`), the canonical halt
  sentinel `ibm704-backend` emits at every function exit.

12 byte-pinned unit tests.
