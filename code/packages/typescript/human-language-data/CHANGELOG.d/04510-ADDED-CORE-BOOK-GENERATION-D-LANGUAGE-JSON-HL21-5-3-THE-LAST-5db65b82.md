### Added - `core/book-generation.d/<language>.json` (HL21 §5.3) — the last shared ledger

- 24 files, one per language plus `_meta.json`, each holding that language's
  slice of all six arrays. A Spanish tranche now edits `spanish.json` and
  collides with nobody working on another language. **This completes HL21's
  three ledgers.**
- **Grouped rather than one file per element**, and it is the only ledger split
  that way. Element-wise this would be over a thousand files nobody opens
  individually (`targets` alone is 1,014). It is also the only ledger with no
  ordinal prefix, and both facts follow from one measurement: all six arrays are
  already contiguous by language and in the same alphabetical order, which is
  *also* sorted `<language>.json` order.
- Round trip byte-exact against the committed file, SHA-256
  `f894e435e9d5ea21d33aebdb2e8e8e53e2ef26fc23342b79159521a4552f812d` — the same
  digest the re-indent commit produced, so this commit changes no bytes of the
  monolith at all. The entire data diff is the 24 new shard files.
- `_meta.json` holds `version`, `sourceBaseUrl` and `scriptSets`. `scriptSets`
  is keyed by *script set*, carries no `language` on any element, and so has no
  per-language home — confirmed, not assumed. No `_keys` needed: the six grouped
  arrays are already a suffix.
- `src/book-cli.ts` now reads through `readMaybeSharded`. It was the last
  non-loader read of any of the three ledgers in `src/`; with the shards as
  source of truth, a direct read would have served the generator a *derived*
  file — correct while `--check` is green and quietly stale the moment it is not.

