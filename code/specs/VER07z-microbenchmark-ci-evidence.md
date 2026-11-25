# VER07z: Preserve complete CI failure evidence

## Purpose

When a repeated CI test command fails, the workflow must retain enough output to
identify the failing assertion and its context. A filtered summary or fixed-size
tail may omit the original failure when later test output is longer.

## Contract

- Keep each test command, repetition count, timeout, captured exit code, and
  existing pass/fail gate unchanged.
- Keep the concise summary used for routine diagnosis.
- On failure, print the complete captured command log to the job output so the
  original assertion remains available in the retained CI log.
- This is an evidence-retention change only. It does not diagnose, waive, or
  relabel any particular test failure.

## Scope

This change applies to the repeated Kiln real-strace test step in
`.github/workflows/ci.yml`. It does not alter Kiln tests or claim that an
unrecovered assertion was a flake.
