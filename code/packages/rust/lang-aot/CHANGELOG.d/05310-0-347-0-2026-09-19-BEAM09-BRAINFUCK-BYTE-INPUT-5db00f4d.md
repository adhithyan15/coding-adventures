## [0.347.0] - 2026-09-19 — BEAM09 Brainfuck byte input

Promote the three Brainfuck stdin rows after real Erlang execution. All six
Brainfuck rows now declare eight backends (48 cells); non-ALGOL declarations
rise from 1677 to 1680. Configure only Brainfuck subprocesses with
`-kernel standard_io_encoding latin1`; text languages keep their host settings.
Raw subprocess tests cover all 256 byte values, repeated EOF and live tape cells.
Replace the old backend-refusal assertion with execution proof.
