### Moved with them
- `data.ts` → `scriptdata.ts`, plus the `Letter` and `ScriptData` types that
  describe the curriculum's script JSON files. They belong beside the pen paths:
  a letter's stroke order is verified against the very font its `ScriptData`
  names, so a test can assert the two agree only if both live here.
  `language-ladder/src/types.ts` re-exports them, so every existing
  `from "./types.ts"` import in the app is unchanged.

