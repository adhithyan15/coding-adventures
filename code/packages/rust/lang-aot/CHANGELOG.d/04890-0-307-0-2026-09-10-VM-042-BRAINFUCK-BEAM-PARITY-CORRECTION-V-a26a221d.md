## 0.307.0 — 2026-09-10 — VM-042 Brainfuck BEAM parity correction (VM-D031)

Reprioritized the post-VM-061 queue (~42 undeclared COBOL BEAM rows, VM-041,
VM-060b, VM-042, VM-058) by actually probing VM-042's premise instead of
implementing it as filed. VM-042 asked to "pin Brainfuck's intentional BEAM
exclusion... for mutable tape operations" — but `iir-to-beam` PR #11343
(2026-08-13) had already added `:atomics`-backed mutable memory generically,
explicitly to unblock Brainfuck, three weeks before VM-042's own text was
written. A scratch probe against real `erl` confirmed all three non-input
Brainfuck matrix rows (`++++++++[>++++++++<-]>+.`, the nested-loop `"HA"`
program, and `"OK"`) already execute correctly through the existing
`store_byte`/`putchar` lowering, with no `iir-to-beam` code change needed.

This is **VM-D031**: `brainfuck-iir-compiler`'s README and
`LANG-VM-FEATURE-COVERAGE.md` both carried a stale "BEAM tape support is
intentionally not supported" claim, correct when written but never revisited
after the generic memory-ops PR landed — rung 3 of the backlog's own
prioritization policy (incorrect status documentation), which the original
VM-042 filing did not catch either.

Promoted the three non-input Brainfuck rows to declare `Beam` (42 → 45
declared cells; `feature_coverage_doc_counts_match_programs_source`'s
expected tuple updated to match). Added
`portable_text_stdout_brainfuck_beam_corpus` (all three execute on real `erl`
with the expected stdout) and
`brainfuck_beam_stdin_rows_refuse_at_backend_not_frontend` (the three STDIN
rows compile through `compile_source_to_iir` — the frontend — without error,
and are refused only by `compile_source_to_beam` — the backend — naming the
missing `getchar` builtin). That refusal/acceptance split is pinned at the
`iir-to-beam` layer too; see that crate's 0.9.1 changelog entry. `getchar`
remains real, separately-scoped host-input work (VM-060b), shared with every
other frontend's undeclared BEAM input rows — not a Brainfuck- or
tape-specific gap.

