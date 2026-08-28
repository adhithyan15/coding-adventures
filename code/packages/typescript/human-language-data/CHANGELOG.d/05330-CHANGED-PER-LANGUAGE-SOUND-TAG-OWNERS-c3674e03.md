### Changed — per-language sound-tag owners

- Replace the cross-language `core/sound-tags.json` aggregate with one strict,
  self-binding owner per registered language under `core/sound-tags.d/`.
- Preserve the historical 27,912-byte registry exactly while rejecting missing,
  extra, malformed, nested, symlinked, non-regular, noncanonical, or mismatched
  owners before validation consumes them.
- Make the language registry an independent completeness gate and remove the
  tracked compatibility aggregate so pronunciation work in different tracks no
  longer shares a file.
