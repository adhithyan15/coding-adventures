### Added - `<track>/curriculum.d/`, and the end of the worst conflict in the corpus (HL21 §5.2)

- 22 of 23 tracks' `curriculum.json` are now stored as three sibling
  directories under `curriculum.d/`, sharing one `_meta.json`:

  ```text
  curriculum.d/_meta.json                          version, language, conceptAliases
  curriculum.d/path/0010-ES-PATH-001.json          the authored ladder
  curriculum.d/extensions/0010-ES-EXT-001-….json   the track's own additions
  curriculum.d/spine/0010-SPINE-MEET-GREET.json    ONE FILE PER SPINE NODE
  ```

- **`spine/` is the prize.** Every content tranche in every track appends to
  `spine[<node>].segments`, and there are only 33 nodes for 23 tracks' worth of
  authors to collide on. Two tranches touching two different nodes now write two
  different files and never meet.
- Round trip verified byte-exact against all 22 committed ledgers before the
  data moved; the monoliths in this commit are unchanged, which is the proof.
  `marwadi` is left on its monolith — its `lessons` arrays are written inline on
  one line, so the bytes do not round-trip. Data identical, reported not
  reformatted, per §8.9.

