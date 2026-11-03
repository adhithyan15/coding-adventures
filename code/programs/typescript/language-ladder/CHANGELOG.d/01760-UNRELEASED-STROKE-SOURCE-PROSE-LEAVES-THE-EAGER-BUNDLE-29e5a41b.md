## Unreleased — stroke-source prose leaves the eager bundle

- **Fix:** Chinese B1 took `data/scripts/chinese.json` to 353
  source-verified characters (789 kB). The `script-data` group's `maxSize`
  cannot split a single JSON module, so the whole file became one 699,737-byte
  eager chunk, and `check:bundle` failed against its 500 kB gate. 410 kB of the
  file is stroke-order provenance prose: each `strokeOrderSource`'s citation
  and `variation` caveat. The app's eager path never shows that prose. It needs
  only the record's existence (`core.ts`, for "verified-ductus") and its `url`
  (`verifiedLetterFont()`). The prose appears on the handwriting card, which
  the lazy handwriting tools fill from script-ductus `src/strokes/chinese.ts`.
  That module imports the file directly.
- **New `script-provenance-plugin.ts`:** a build-only, `enforce: "pre"`
  plugin.
  - When script-ductus's `scriptdata.ts` imports a listed inventory
    (`SLIM_SCRIPTS`, currently `chinese`), it gets a virtual copy in which every
    `strokeOrderSource` is cut to `{ url }`.
  - Every other importer, the stroke modules included, still gets the real
    file.
  - The `script-data` chunk group now uses the plugin's tested predicate,
    `isEagerScriptDataModuleId`. It keeps everything the old regex admitted,
    adds the slim copies, and leaves out the full files of slimmed inventories,
    which then ride with the lazy stroke modules.
  - Arabic is deliberately not listed: its stroke module reads sources through
    `scriptdata`'s `SCRIPTS`, so slimming it would blank its citations. A test
    pins that rule.
- **Verified:** a minimal build was run with the real `scriptdata.ts`, the
  Chinese stroke module behind `import()`, both plugins and the same chunk
  group.
  - Without the plugin it reproduces CI's chunk byte for byte:
    `script-data-CAP5vG0m.js`, 699,737 bytes.
  - With the plugin, the largest eager chunk is 353,040 bytes, and no eager
    chunk holds a citation. The full record, prose included, is only in the
    dynamically imported Chinese chunk.
  - A full local `vite build` needs about 13.4 GB, more than the authoring
    container's shell memory limit, so CI's `check:bundle` remains the
    full-app check.
- **Tests:** `tests/script-provenance-plugin.test.ts` (13 cases) covers:
  - slimming at any depth, with other fields untouched and a `__proto__` data key kept as an own property;
  - the group predicate, admitting and excluding the right ids;
  - resolution: only `scriptdata.ts`, only listed scripts, nothing outside the
    curriculum root;
  - the rule that a slimmed script's stroke module imports its own file and
    does not read `scriptdata`.
