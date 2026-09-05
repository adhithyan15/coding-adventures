# Script inventory evidence ownership

Each `*.evidence.ts` module in this directory owns assertions for exactly one
script inventory entry or one semantically coupled group of entries. A script
author may assert that their own glyphs leave the measured gap set, but must not
pin the corpus-wide highest-ranked gap here.

The exact corpus queue is owned by the sibling
`tests/script-inventory-queue.ts` module. It receives the already-loaded corpus
context from `integration.test.ts`, so changing its expectation neither edits an
unrelated script module nor adds another `loadEverything()` call.
