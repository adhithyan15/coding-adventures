## Changed — sharded Language Ladder authoring surfaces (#13211)

- Move all 34 glyph evidence cases and their 287 assertions into script-owned
  modules discovered by one stable aggregator, preserving their exact execution
  order and sharing one loaded script corpus.
- Replace this shared changelog monolith with append-only level-2 fragments,
  backed by the repository document sharder and deletion/drift CI guards.

