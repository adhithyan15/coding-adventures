## 0.4.0 (patch, no version bump) — TFV duplicate-id check is linear

- `tfv::parse_file` detects a repeated record id with a hash set instead of
  scanning every earlier record.  The scan was quadratic, so a table of 100,000
  rows (VER04j WX-E allows that many) cost about 5 * 10^9 comparisons.  The
  accepted language, the error text and the error line are unchanged; a unit test
  parses 100,000 rows and still reports a duplicate on its own line.
