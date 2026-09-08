### Fixed — a hand-written pronunciation reference can no longer pass unseen

- Seventeen tracks shipped a hand-written `appendix-pronunciation.tex` for months
  with every gate green, because nothing could see them. `shard-cli`'s
  cross-ledger check builds the generated-reference set from files that begin
  `% GENERATED FILE.` and requires it to equal the ledger — and those two agree
  perfectly when a track's appendix is hand-authored: the file contributes
  nothing, the ledger owns nothing, and an equality between two empties holds
  over a chapter no generator has ever touched. `handwritten_parity.py` could not
  see them either; it reads the ledger's `handwritten` block, and an appendix
  nobody declared is in no block at all.
- `shard-cli` now states what that equality cannot: every registered track has
  exactly one pronunciation reference and it is generated. All 23 satisfy it, so
  it is a promise rather than a snapshot, and it names the offending tracks.
- `book-cli.test.ts` asserted the `% GENERATED FILE.` stamp on a hardcoded list
  of five languages — the five that had been missing at the time. A literal list
  is the wrong instrument once the answer is "all of them"; it is derived from the
  registry now.
- `grouped-shards.test.ts`'s `reference-appendices.d` count collapses from a
  floor-and-ceiling ratchet to `toHaveLength(tracks)`, as its own comment said it
  would once the retirement finished.
