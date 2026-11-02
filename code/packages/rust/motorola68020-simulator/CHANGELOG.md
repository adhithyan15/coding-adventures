# Changelog

## Unreleased

- Add the bounded Spec 07at MC68020 functional simulator with complete checked
  state, full extension-word `MOVE.L` addressing, problem-state extensions,
  typed transactional lifecycle, traces, and inherited MC68000 differentials.
- Reject every reserved full-extension index-suppressed indirect encoding before
  consuming displacement or pointer data, freeze instruction bytes before
  self-modifying execution, and normalize the false `TRAPcc` trace mnemonic to
  uppercase `TRAPCC`.
- Permit architecturally valid odd-address word and longword data transfers,
  including memory-indirect pointer reads, while retaining even instruction
  and extension-word fetches and whole-range atomic data checks.
