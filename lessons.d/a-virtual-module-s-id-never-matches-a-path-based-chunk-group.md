---
category: TypeScript / JavaScript
---

# A virtual module's id never matches a path-based chunk group

**What went wrong.** The Persian digits PR (#17194) added twenty digit rows to
the shard-native Persian and Urdu script inventories. The app's `validate` job
then failed its bundle gate: the eager `index` chunk was 507,393 bytes against
a 500,000 limit. language-ladder's `vite.config.ts` already routes script
inventories into size-capped `script-data` chunks, but its group matched only
the file path `learning/human-languages/data/scripts/`. Shard-native
inventories reach the app through script-ductus's virtual module
(`\0virtual:script-ductus-inventories`), whose id contains no such path, so
the whole merged corpus had been sitting in `index` all along. The digits were
only the bytes that tipped it over.

**Fix.** The `script-data` group's test now also matches
`virtual:script-ductus-inventories`. `index` fell to 169,553 bytes; the
largest eager chunk is now 337,951 bytes.

**Next time.** When a bundle gate trips on a small change, find which module
put the bytes in that chunk before trimming the change. A chunk group keyed on
file paths silently misses every virtual or generated module; key it on the
module id the bundler actually sees.
