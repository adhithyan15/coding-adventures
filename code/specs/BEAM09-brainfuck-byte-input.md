# BEAM09: Brainfuck byte input and EOF

Status: implemented and locally validated 2026-09-19; PR checks pending.

## Reprioritization after restart

Main 38eff41652 includes BEAM08 (#15467), all 51 BASIC and all 49 Twig
BEAM corpus rows. The paused BASIC string probe is superseded by #15147;
its local work is preserved on codex/lang-vm-basic-beam-string-probe.
No open LANG implementation PR overlaps this item at selection time.

1. Complete the three existing Brainfuck input rows on BEAM (this item).
2. Reconcile remaining non-ALGOL backlog against executed evidence, then select
   a bounded semantic gap (including COBOL region intersection and encoded CLR
   input). Oct/Nib portable corpus coverage is not full historical CPU fidelity.
3. Keep ALGOL with its separate owner. Reprioritize discoveries before each PR.

## Contract and design

Brainfuck comma reads exactly one unsigned byte; EOF supplies zero. Repeated
reads and loop iterations must preserve the tape handle, pointer and live values.
Add getchar to backend validation and imported-call liveness. Lower it through
io:get_chars('', 1), map eof to zero, and extract the returned one-element list.
Unexpected I/O error terms must trap rather than masquerade as EOF.

The host owns the standard I/O encoding. Launch Brainfuck's Erlang subprocess
with -kernel standard_io_encoding latin1 so both input and output are raw bytes.
Leave text languages' host settings unchanged. This setting is necessary even
for output-only Brainfuck programs: modern OTP otherwise UTF-8 encodes bytes
128..255. Document it for callers outside the matrix harness.
Reference: https://www.erlang.org/doc/apps/stdlib/io.html (standard I/O encoding).
Do not conflate the byte stream with BASIC/FLOW-MATIC line lookahead.

## Acceptance

- Execute the existing three stdin rows on real Erlang before declaring Beam.
- Replace the old explicit-refusal test with execution proof.
- Exercise all 256 byte values using a fixed-length echo (NUL is data), empty
  and repeated EOF, no final newline, and live tape state across reads.
- Compare raw subprocess stdout for the byte probe; do not decode it lossily.
- Keep detected-runtime failures hard; missing Erlang alone permits a skip.
- Run backend tests, all BEAM matrix tests, coverage-count guard and Clippy.
- Promote only proven cells, update coverage, README and changelogs, security
  review, publish a ready PR, monitor exact-head checks, auto-merge when green.

## Probe discovery

The initial unrolled 256-byte echo hit the existing backend 255-variable limit.
Use `-[>,.<-]>,.` (255 loop iterations plus one final read) for byte coverage.
Register allocation beyond that limit is a separate backlog candidate; it does
not block the byte-input contract and is lower priority than remaining semantic
gaps. The three original input rows already pass real Erlang before promotion.

## Validation results

All three original input rows passed before promotion. The raw-byte loop,
repeated EOF and two-live-tape-cell probes passed against real Erlang. All 35
BEAM matrix tests and the complete iir-to-beam suite passed, as did the coverage
count guard and Clippy for both changed Rust packages. Declarations are now
Brainfuck 6 rows / 48 cells and non-ALGOL 210 rows / 1680 cells. The other seven
backend columns were not rerun in this slice; do not describe declarations as
1680 freshly executed cells.
