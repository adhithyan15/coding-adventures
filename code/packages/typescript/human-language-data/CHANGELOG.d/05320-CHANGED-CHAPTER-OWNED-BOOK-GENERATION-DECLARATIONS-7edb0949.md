### Changed — chapter-owned book-generation declarations

- Replace the 23 per-language `book-generation.d/*.json` slices with stable
  chapter owners and independently owned backmatter and script-set sections.
- Reconstruct the historical manifest byte for byte while enforcing exact
  registry, chapter-capability, generated-book, and generated-narration identity
  equality.
- Reject unsafe, unexpected, duplicate, nested, symlinked, non-regular, or
  mismatched owners without restoring a tracked compatibility aggregate.
