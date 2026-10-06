---
category: Testing & coverage
---

# A corpus test shard needs an eight-hex suffix before the suite can load it

The Punjabi named-reader corpus test initially used a descriptive
`*.case.ts` filename without the required eight-hex suffix. Vitest found the
language suite, but its shard loader rejected the filename before any test
ran. That failure said nothing about the lesson contract.

`tests/corpus/corpus-test-shards.ts` requires
`<lowercase-kebab-title>-<eight-lowercase-hex>.case.ts`. Rename the owner to
that shape before running the language suite. Then execute the actual
`tests/corpus/punjabi.test.ts` entry point with a title filter; running a
`.case.ts` file directly finds no tests because it is imported by the suite.
