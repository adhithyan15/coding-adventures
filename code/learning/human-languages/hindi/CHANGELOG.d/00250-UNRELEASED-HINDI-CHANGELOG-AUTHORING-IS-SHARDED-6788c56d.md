## Unreleased — Hindi changelog authoring is sharded

The Hindi track's measured 24-section baseline and the concurrently landed
child-form entry now live under `CHANGELOG.d/` as stable level-2 fragments. The
aggregate is an ignored local view that can be rebuilt with the
human-language-data `unshard:docs` command; authors add uniquely ranked
fragments instead of editing a shared top-of-file insertion point. A byte-exact
migration gate preserves the measured baseline, and the ordinary document-shard
gate rejects stale aggregates and malformed fragments.

This removes the same-language collision point measured in issue #14245: Hindi's
monolith was touched by 23 recent curriculum commits even after the corpus-wide
ledgers and Language Ladder changelog had already been sharded.
