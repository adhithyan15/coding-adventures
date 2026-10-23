---
category: Testing & coverage
---

# Vitest corpus `.case.ts` files run through their language entrypoint

The human-language corpus shards are loaded with `import.meta.glob`, so a single
case is exercised by running the matching `tests/corpus/<language>.test.ts`
entrypoint rather than passing the `.case.ts` path directly to Vitest.
