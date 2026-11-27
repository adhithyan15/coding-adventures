# Machine-layer probe fixtures

`boards.json` contains the `ascii-console-demo-v1` instruction bytes and runs
that identical image and synthetic CPU on two portable board maps. The cases
expect identical console bytes with different memory capacity and mapped
console addresses. They also record the bounded instruction budgets, final CPU
registers, instruction sequence, and data-only console write effects.
`verify_boards.mjs` reads this fixture outside the package root, runs the
library-only board simulator, and compares both traces for build validation.
