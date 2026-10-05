### Changed — Kannada pins for the tta, nna, sha, ssa, dha, bha, pha, kha, gha and ddha stroke orders

- `tests/script-inventories/kannada.evidence.ts` pins the ten consonant
  rows ಟ, ಣ, ಶ, ಷ, ಧ, ಭ, ಫ, ಖ, ಘ and ಢ: role `syllable`, pen lifts (0, 0,
  1, 3, 2, 1, 3, 0, 3, 2), the exact stroke-order sentences, the Commons URL,
  and the citation's frame count and duration. The variation must name the
  mirror copy and what was compared: size and bytes for most, the pixel size
  alone for ಖ and ಘ, and for ಭ that no listed size was available. For ಟ, ಧ
  and ಢ it must name the slug ("ta", "dhha", "dda") and the letter the
  neighbouring slug draws, so a later edit cannot cite ತ's, ದ's or ಡ's
  animation by mistake.
- For ಭ and ಘ, which lift less often than their animations, the pins also
  require the Omniglot citation, that its writers are non-native copyists
  whose counts are read only as a ceiling, the copyists' shares, and where
  the path joins its runs; the number of "lift" steps must equal the lifts.
- `tests/filmstrip-target-counts/kannada.json`: 36 -> 46. The new targets
  are KA-S151-letter-tta, KA-S164-letter-nna, KA-S165-letter-sha,
  KA-S166-letter-ssa, KA-S167-letter-dha, KA-S168-letter-bha,
  KA-S169-letter-pha, KA-S170-letter-kha, KA-S171-letter-gha and
  KA-S172-letter-ddha.
- Regenerated: the ten filmstrip SVGs, the Kannada figure hashes, the books
  and narration for chapters 73, 79 and 80, and the lesson modality of the
  ten letter lessons.
