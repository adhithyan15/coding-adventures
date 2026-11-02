---
package: *
lane: rust
rung: E5
status: open
---

# There is no Rust grammar or parser in the repository

Proofs reason over the Warrant IR, and each language reaches it through a
front end built on that language's grammar and parser. The repository has
`.tokens`/`.grammar` files and parsers for C, Python, TypeScript and many
others, but none for Rust -- the pilot's language.

**Open:** VER26 step 22 adds a Rust-subset grammar covering exactly what
`ct-compare` uses, generated through `grammar-tools` like every other grammar.
