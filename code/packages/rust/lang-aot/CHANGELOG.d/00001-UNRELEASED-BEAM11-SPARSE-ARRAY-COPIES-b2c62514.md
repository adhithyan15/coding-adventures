## Unreleased — BEAM11 sparse ALGOL array-copy proofs

`tests/beam_array_len.rs` now executes six existing ALGOL matrix sources on
real Erlang that pass sparse real or string arrays by value. All six returned
42 after the shared `iir-to-beam` default-read repair. Before BEAM11, copying
an unwritten source cell raised `badarg` on this substrate. These tests make
the observed VM-071 defect a continuing executable regression gate; they do
not add BEAM to ALGOL's declared matrix columns, which the ALGOL owner tracks
separately.
