### Fixed — every fused sign pair cites its source

- `src/figure-targets.ts`: new `FUSED_SIGN_PAIR_SOURCES` (and the
  `FusedSignPairSource` type) groups the consonant + sign pairs the filmstrip
  composer refuses under the source that shows them fused: Unicode 17.0
  §12.6.3 Figure 12-21 for Tamil டி டீ லீ, and three Noto Sans Gujarati 2.106
  GSUB citations for Gujarati (`blws` lookup 90 stem forms, 44 pairs;
  `blws` lookup 89 ligatures રુ રૂ ણુ; `psts` lookups 94 to 96 and 106 for
  જ and ૹ with ા ી ો ૌ). Each has an HTTPS URL. `FUSED_SIGN_PAIRS` is now
  built from that table alone, so its contents are unchanged (Tamil 3,
  Gujarati 54) and no pair can exist without a citation.
- New `tests/figure-targets/fused-sign-pairs-are-cited-*.case.ts`: every
  pair's letter and sign are named in its citation, every letter a citation
  names has a pair, each source's pair count and feature are pinned, and the
  bundled Gujarati font still carries the cited version string. Dropping
  லீ or adding an uncited லி fails it.
- The lookup numbers were re-read from the bundled font with fontTools for
  this change. HL06 and the README say where the citations live.
