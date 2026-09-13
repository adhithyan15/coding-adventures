# A slot's consumers use its camelCase alias, so grepping the kebab key misses them

Renaming a Mosaic slot `deck-names` → `deck-rows`, I swept the repo for
`"deck-names"` and found every Rust site. CI then failed on
`engram-wasm/js/smoke.mjs`, which asserts `demo.props.deckNames` — the host
converts kebab-case prop keys to camelCase, so **no consumer outside the
`.mil`/`.mll` files spells the name the way the declaration does**.

The Kotlin, Swift, Dart and C# consumers are the same: `deckNames`,
`DeckNamesProperty`, `let deckNames: [String]`. A grep for the declared key
finds the declaration and nothing that reads it.

When renaming a slot, grep **both** spellings, and include the PascalCase one
for XAML:

```bash
git grep -n 'deck-names\|deckNames\|DeckNames'
```

Note that two of the consumer suites — the JS smoke test and
`engram-app/tests/package_compiles.rs` — are outside the Rust workspace, so
`cargo test --workspace` does not reach either.
