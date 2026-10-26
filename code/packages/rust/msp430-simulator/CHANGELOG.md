# Changelog

## 0.1.0

- Add the configuration-pinned classic MSP430F2274 CPU model.
- Implement all classic Format I, II, and III families and addressing modes.
- Add checked little-endian memory, stack, PC/SP/SR canonicalization, and the
  deterministic repository halt sentinel.
- Add complete-state traces and typed transactional load, step, run, execute,
  snapshot, and restore operations.
- Add exhaustive decoder, addressing, flag, lifecycle, and rollback tests.
- Enforce the SLAU144K source-only boundary for all R2/R3 constant-generator
  forms used by modifying Format II operations while retaining legal
  `PUSH`/`CALL` sources.
- Report modifying Format II addressing through `destination_mode` and
  `PUSH`/`CALL` addressing through `source_mode`.
- Add deterministic allocation-failure seams and exact rollback witnesses for
  clone, load, fetch bytes/words, trace retention, and final trace state.
