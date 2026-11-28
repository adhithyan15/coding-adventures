# DB-E41 — Route the interrupted NAND erase vector

Give the admitted Go DB04d package its exact shared block-erase vector.
The rule is `repository-database-media-nand-block-erase-cuts-fixture-inputs`.
Its sole exact root is `code/packages/go/nand-block-erase-cuts` and its
sole input is
`code/specs/fixtures/database-media-v1/nand-block-erase-cuts.json`.
The runtime and fixture enter this slice at the accepted revision
`652067a85f17`. [Block erase spec](DB04d-nand-block-erase-cuts.md)

Compare direct Go-only and unfiltered selection, exact file deletion
and nearby nonmatching paths. Check fallback content hashing and Kiln
inputs while editing the block-erase vector, then repeat with the
interrupted program vector and the ordinary NAND and HDD vectors.
Each edits its own direct readers. DB-E50 will cover dependency
propagation after the FTL block adapter joins the routing stack.

Run `sync-boundary` to regenerate projections, digest references and
census counts, then require a clean `sync-boundary --check`. Run the
native Go build-tool tests and vet, C# and Swift conformance, focused
Python build-tool tests and corpus validation.
