# Changelog

## Unreleased

- Add `cases.tfv`, the corpus in TF01's line format, generated from
  `cases.json` by `code/scripts/ct_compare_conformance.py --write`. The
  validator now fails when the committed file is stale. The cases themselves
  are unchanged.

## 1.0.0 — 2026-10-05

- Freeze CT01's closed, versioned byte and unsigned-64 case transport.
- Add 21 independently validated functional and public-work cases, including
  public length rejection, high-bit bytes, a 256-byte comparison, both
  selection choices, fixed widths, and sign/low/middle-bit u64 mismatches.
- Separate functional conformance and source/control-flow review from any
  universal wall-clock or hardware constant-time claim.
