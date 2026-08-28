## Changed — nested Ductus shard bundle routing

- Keep nested per-glyph Script Ductus source modules in Language Ladder's
  handwriting-tools chunk on POSIX and Windows build paths.
- Test the pure package-path boundary so future owner depth changes cannot
  silently inflate the eager application chunk.
