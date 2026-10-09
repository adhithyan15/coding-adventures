## Unreleased — shard-native script inventories join the script-data chunks

The `script-data` chunk group in `vite.config.ts` matched only files under
`data/scripts/`, so the shard-native inventories (served through
script-ductus's `virtual:script-ductus-inventories` module) were bundled into
the eager `index` chunk. Adding the Persian and Urdu digit rows took `index`
to 507,393 bytes, over the 500 kB bundle gate. The group now matches the
virtual module too: `index` is 169,553 bytes and the largest eager chunk is
337,951. Only chunking changes; the app loads the same data.
