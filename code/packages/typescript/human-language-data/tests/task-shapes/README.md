# Language-owned task-shape tests

Put exact task-shape contracts in `<language>.test.ts` here. A language PR may
add as little as one test file; it should not append language-specific literals,
counts, or inventory entries to `../task-shapes.test.ts`.

The shared suite owns discovery, ordering, backlog arithmetic, parsing, and
cross-language invariants. This folder owns the exact content of each language's
assessment envelope, so parallel language PRs add independent files instead of
editing one global ledger.
