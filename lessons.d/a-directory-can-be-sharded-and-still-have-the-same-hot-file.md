# A directory can be sharded and still have the same hot file

`core/book-generation.d/` had replaced the corpus monolith, but it grouped all six declaration
arrays into one file per language. That removed conflicts between Spanish and Gujarati work while
preserving every conflict between two Spanish chapter tranches. The `.d/` suffix described the
storage mechanism; it did not prove that the ownership unit matched the unit of parallel work.

Splitting every array by its real identity removes the remaining collision. Generated and
handwritten chapters use `<language>-<NNNN>.json`; backmatter uses language plus output basename;
script sets use a stable spaced ordinal plus id because their authored object-key order is not
alphabetical. The fold can still reproduce the old 188,438-byte document exactly without keeping
that document, or a language-sized projection of it, as a tracked file.

The seam also needs relational checks. A locally valid chapter owner may still be a ghost relative
to the active registry or capability ledger. Targets must exactly equal generated-book hash
identities, while targets plus handwritten chapters must exactly equal both narration-hash and
chapter-capability identities. Subset checks miss stale or orphaned owners in the other direction.
Backmatter needs the same treatment: registry-derived standard outputs and generated-reference
markers provide an expectation outside the owner directory, while every script set must have a
surviving declaration that uses it.

The migration path is part of the ownership design. Build and validate the complete new tree under
a private temporary directory, then rename it into place; never delete the legacy directory before
the replacement exists. Also reject a resurrected aggregate whenever canonical nested owners are
already present, or a stale aggregate can overwrite the very independent edits sharding protects.

**Rule:** measure contention at the file authors actually edit, not at the directory containing it.
Choose a stable filename for each independent semantic owner, preserve authored order in the
filename where necessary, and state cross-ledger invariants as set equality in both directions.
