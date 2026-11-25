---
category: Compiler / VM / language pipeline
---

# A syntax-corpus positive case must be accepted by every parser that reads the corpus, not only the grammar

**What went wrong.** ADL-001a added the `profile algorithm` header to `code/grammars/warrant/` and four cases to `code/specs/fixtures/warrant-syntax-v1/cases.json`. The corpus is not read only by the grammar-driven `self_corpus` runner. It is also read by the guarded Rust parser (`tests/guarded_parser.rs`, which asserts every `accept: true` case parses and the count is exactly 57), by two hand-written TypeScript parsers (`warrant-compiler` and `test-bootstrap`, both hard-coding 57 and the v0 pure profile), and by a `.tokens`/`.grammar` sha256 pin in `tests/wxd_syntax.rs`. A grammar-only positive (`warrant m profile algorithm`, which the guarded parser refuses with W0801 because a profile needs `v1`) made `guarded_parser` fail, and the accepted v1 header made both TypeScript suites fail.

**Fix.** Keep a corpus positive only if the guarded parser also accepts it; put "grammar accepts, guarded parser refuses" cases in a diagnostics corpus instead (`warrant-profile-algorithm-v1`). Teach each hand-written parser to refuse the new clause by name rather than by accident (`UnsupportedProfile`), and make its corpus test expect that refusal. Re-pin the grammar hashes with the reason.

**Do differently.** Before editing a shared corpus, `grep -rl <corpus-dir-name> code` outside `.md` files and list every reader; run each reader's own BUILD, not only the package you changed. Count assertions (`57`) are readers too.
