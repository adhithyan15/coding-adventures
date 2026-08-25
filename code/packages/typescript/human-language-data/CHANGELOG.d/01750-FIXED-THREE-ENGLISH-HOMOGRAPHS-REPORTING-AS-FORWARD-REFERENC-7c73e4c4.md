### Fixed - three English homographs reporting as forward references (HL-C103)

- `comes`, `hand` and `regular` are ordinary English words that are also target
  headwords, so sentences like "*comer* **comes** from Latin *comedere*" and
  "**Regular** stress: TAR-de" reported the Spanish word as a forward reference
  from a lesson that was writing English.
- Added by **census, not guesswork**: of 423 forward references, 368 matched via
  emphasis and 55 in plain prose only; 18 of those were pure-ASCII candidates
  and exactly three were English. The other 15 are genuine and must keep
  reporting -- a list built from a plausible wordlist would have suppressed
  them. The census method is now recorded in `continuity.ts`.
- Records that the structural alternative -- guard the plain path only, and
  trust emphasis to mean "target language" -- was tried and is wrong: authors
  emphasise English for stress too.
- Also drops `tres` from an activity's `accepted` list, where it offered credit
  for a word the reader has not met.
- Forward references **423 -> 418**.

