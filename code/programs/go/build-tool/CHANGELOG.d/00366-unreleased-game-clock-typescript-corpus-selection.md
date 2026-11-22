## Unreleased — GAME10 corpus selects the TypeScript consumer

- Registered `typescript/game-clock` as a repository-input consumer of the exact
  `game-clock-v1` fixture directory. Warrant-only changes continue to select the
  Rust clock and global Warrant checker, not TypeScript.
- Added language-filtered selector checks and versioned-path negative controls.
