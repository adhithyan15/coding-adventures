### Not done, and why - three tracks and the deleted monolith

- **`french`, `japanese` and `marwadi` keep their monoliths.** Their committed
  `chapters.json` is hand-formatted with inline one-line arrays that
  `JSON.stringify(x, null, 2)` expands over three lines. The *data* is identical
  — checked by deep comparison — but the *bytes* are not, so they cannot migrate
  without a reformatting commit that rewrites lines nobody asked to change.
  HL21's rule is to report a ledger that does not round-trip rather than quietly
  reformat it into agreement with the serialiser. The loader's fallback reads
  them unchanged.
- **The monolith is kept rather than deleted, and that is a compromise.**
  Deleting it is what would remove the conflict outright; keeping it means every
  tranche still regenerates it and still collides on that one file — though on
  HL21 §3's terms, where the fix is `npm run unshard` rather than a hand-merge.
  The blocker is the browser: `language-ladder` reads these ledgers through
  `import.meta.glob`, and while a glob's *modules* are lazy, its *key table* is
  eager code in the importing module. Sharding took that table from 23 entries
  to ~1,020 and grew the app's largest eager chunk by 191 kB — 312,216 to
  503,765 — through the hard 500 kB budget in `scripts/check-bundle.mjs`. That
  budget is a ceiling on debt and was not raised. Moving the capability fields
  into the generated hash manifest the app already loads would dodge it, at the
  cost of a real check: the app recomputes each chapter's fingerprint from the
  currently authored capability, so a capability edited without regenerating
  reads as `stale`; sourcing it from the same generated file as the hash would
  make that comparison agree with itself.

