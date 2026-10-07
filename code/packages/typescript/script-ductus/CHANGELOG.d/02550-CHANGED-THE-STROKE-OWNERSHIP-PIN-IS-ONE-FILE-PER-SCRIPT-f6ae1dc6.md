### Changed — the stroke-ownership pin is one file per script

`tests/stroke-ownership.test.ts` pinned the whole `DUCTUS` registry with one
literal (`keys`, `keyHash`, `nonTamilDataHash`, `counts`, and the two
shared-identity values). Every stroke or filmstrip PR in any script had to
rewrite it, so a Kannada PR and a Malayalam PR in flight together always
conflicted on the same lines (#12118, #13193). The literal and its 400-line
comment log of every past move are gone; that history lives in git and in the
CHANGELOG.d fragments that made each move.

- **`tests/stroke-ownership/<script>.json`** — one pin per canonical script
  (16 today): `count`, `runs` (the length of each contiguous block the script
  occupies in registry order), `keyHash` (its keys in order) and `dataHash`
  (its entries' data). `tamil.json` has no `dataHash`, because Tamil data is
  pinned glyph by glyph in `tests/strokes/tamil/`, exactly as the old
  `nonTamilDataHash` left it out.
- **`tests/stroke-ownership/_registry.json`** — the facts no script owns: the
  script of each block in order (`scriptRuns`), and the Arabic family's shared
  stroke objects (`sharedIdentityGroups`, `sharedIdentityHash`). It moves only
  when a script joins or the Arabic family's interleaving changes.
- **`tests/stroke-ownership-pins.ts`** — measures, loads and writes the pins.
  The loader fails closed like human-language-data's per-track pin loaders:
  real directory, real files, lowercase script names, exact field sets.
- **`npm run generate:stroke-ownership`** (`vitest run
  tests/stroke-ownership.test.ts --mode write`) rewrites the pins and deletes
  the pin of a script that no longer exists — the same `--mode write` switch
  as `generate:filmstrip-ledger`. Without it the test only compares.

The gate is exactly as strict as before. A new test rebuilds the full ordered
key list from `scriptRuns`, each script's `runs` and its ordered keys, so the
per-script pins still determine the old global key hash; at migration the
rebuilt list hashed to the retired `2c4ee249…` and the non-Tamil data to the
retired `ae902d92…`, with all 16 counts and both shared-identity values equal.
A missing pin for a new script, a stale pin for a removed one, or any moved
value fails and names the file and the field (`MOVED kannada.dataHash: … ->
…`). A second new test shows on copies of the real registry that editing,
appending or removing a Kannada glyph moves `kannada.json` alone, that editing
Tamil data moves nothing, and that a new script moves its own pin plus
`_registry.json`.
