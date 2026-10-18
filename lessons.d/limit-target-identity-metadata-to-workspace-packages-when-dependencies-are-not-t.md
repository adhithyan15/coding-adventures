---
category: Testing & coverage
---

# Limit target-identity metadata to workspace packages when dependencies are not the validation subject

An owned-source retained-corpus baseline used full offline workspace metadata only to locate one integration-test target. Cargo tried to download an uncached Android dependency and exited101 before any native test ran. All source/index/tool guards remained true. Preserve that actual failure and raw stderr; it is neither a test failure nor acceptance.

Use `cargo metadata --offline --locked --format-version 1 --no-deps` for workspace package and target identity, as the previously accepted runner did. Let the actual locked offline focused test compile its real dependency cone. Use a distinct receipt and target for the corrected attempt; do not relabel the failed attempt. Dependency graph audits remain separate evidence.

The first lesson CLI attempt used an invented category and was refused. Read `lessons.d/_meta.md` and use the literal `Testing & coverage` category.
