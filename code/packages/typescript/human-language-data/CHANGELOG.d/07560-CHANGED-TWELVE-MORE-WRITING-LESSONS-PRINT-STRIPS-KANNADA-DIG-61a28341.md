### Changed — twelve more writing lessons print strips: Kannada digits ೧-೯ and ಂ, Malayalam ജ and ൈ

- **Kannada 47 -> 57.** The strips are KA-S123-sign-anusvara and the digit
  lessons KA-S140 to KA-S148 (೧-೯). Their stroke order cites Chimple's
  tracing lessons: one source, facts only, medium confidence. The digit rows
  and the anusvara's mark record in `data/scripts/kannada.json` now carry
  `strokeOrder`, `penLifts` and `strokeOrderSource`. ೦ (KA-S149) has no
  source and stays without a strip.
- **Kannada ಃ redrawn.** KA-S132's strip now draws the upper dot first, as
  its source does. The earlier path drew the lower dot first.
- **Malayalam 58 -> 60.** The strips are ML-S145-letter-ja (ജ, Moag's six
  movements) and ML-W114-ai-sign (ൈ, two coils of െ, one lift). ൈ is a new
  mark record in `malayalam.json`. It claims no written order against its
  consonant, so words with it stay refused.
- **`validate`:** a digit row that lists `strokeOrder` must now carry
  `penLifts` and `strokeOrderSource` together, as a letter row must. Comments
  no longer say that digit rows never carry a ductus. A new `validate.test.ts`
  case covers a lone lift count and a malformed source.
- **Lesson prose:** the twelve lessons' Writing blocks now point the learner
  at the numbered strip instead of saying the book gives no stroke order.
- **Pins:**
  - `filmstrip-target-counts`: kannada 57, malayalam 60.
  - `script-inventories/kannada.evidence.ts`: pins the digit rows, ಂ's
    ductus and ಃ's new captions and Chimple corroboration.
  - `malayalam.evidence.ts`: adds ജ to the Moag rows, and pins ൈ and the
    marks order.
- **Regenerated:** figures, figure hashes, book chapters and hashes,
  narration and hashes, and the modality manifests of the twelve lessons.
