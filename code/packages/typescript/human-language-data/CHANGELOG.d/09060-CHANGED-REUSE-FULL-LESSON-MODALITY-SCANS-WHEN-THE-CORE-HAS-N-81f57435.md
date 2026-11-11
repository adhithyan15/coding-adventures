### Changed — reuse full-lesson modality scans when the core has no detachable blocks

- Reuse the already-derived script-block, sight-cue, and table-width results for
  a core whose text is byte-for-byte the full lesson. Lessons with detachable
  writing or script sections still scan their distinct core text independently.
- Keep the full-lesson unspeakable-table rule and the core's width rule distinct;
  no modality, reason, manifest byte, or source-cache contract changes.
- The profiled manifest derivation fell from about 5.6 to 3.3 seconds on the
  same Windows corpus with warm lesson-cache hits. The unchanged cold focused
  suite passed 137 tests across three files; full-suite and CI validation are
  still required for #19163.
