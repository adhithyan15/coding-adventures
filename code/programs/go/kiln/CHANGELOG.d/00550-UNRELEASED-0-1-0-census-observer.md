## [0.1.0] - UNRELEASED

### Added (census PR B: the in-band classifier census, `census.json`)

- `traced.Census`, an optional observer on `traced.Input.Census`, sees every
  path-carrying reason (`host-read`, `host-read:setup-modified`,
  `host-read:tool-not-keyed`, `writes-tracked`, `trace-visible`, `state-write`,
  `network`, `daemon`) BEFORE the 16-per-kind cap, and the path behind every
  `trace-incomplete:proc-path` (which carries none in the record). Paths use the
  existing redact/sanitize helpers; at most 4096 distinct (kind, path) pairs per
  action, with counts and an overflow counter. It never feeds back: every
  existing classifier fixture is now also run with a census and the records
  must be byte-identical.
- `kiln record classify-action` always attaches it and, after record.json is
  written, writes `traces/<sha256(action)>/census.json` (schema
  `kiln-record-census/1`: total, overflow, entries of kind/path/count), best
  effort (a failure, even a panic, changes neither the verdict nor the exit
  status), atomically (temp + rename) and bounded to 512 KiB. Nothing reads it
  for a verdict; it is in no shard and in nothing sealed or hashed.
- `collect`'s action-directory scan skips `census.json` and a stray
  `census.json.tmp-*` (they were otherwise "an unexpected entry" and made the
  action unknown). `step`'s order check already ignored non-numeric names; a
  test pins that `.rerun`, `.overlap` and sequencing are unaffected.
- KILN04 gains the "Census side file" paragraph.
