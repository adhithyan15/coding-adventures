# Editing human-language LESSON files breaks the language-ladder APP's tests, not just the data package's

- **A content change under `code/learning/human-languages/*/lessons/` crosses two packages.** `code/packages/typescript/human-language-data` parses the lessons, but `code/programs/typescript/language-ladder` ALSO loads them at build time via `import.meta.glob` and pins facts about them. Running only the data-package suite is not enough, and CI will catch what you skipped.
- Concretely (HL-C18A, PR #9982): splitting fifteen over-budget Spanish lessons into thirty-three micro-lessons moved the per-chapter lesson counts hardcoded in `language-ladder/tests/bookhashes.test.ts` (ch3 12→14, ch4 13→15, ch6 7→9). The data package was green, `npm run check:books` was clean, and the generated hash manifest was regenerated correctly — the only stale thing was the app-side pin. Same shape bit `modality.test.ts` and `integration.test.ts` on other lesson-adding PRs.
- **Rule: after ANY change to lesson files, script data, or `core/*.json`, run BOTH suites.**
  ```
  cd code/packages/typescript/human-language-data && npx vitest run
  cd code/programs/typescript/language-ladder && npm install && npx vitest run
  ```
  The app suite takes ~85s and needs the `file:` dep installed first.
- When a pinned corpus count legitimately moves, **update the pin with a comment saying why** — never delete or loosen the assertion. The surrounding assertions (hash matches the browser-loaded AST, chapter reports `synced`) are the real gate and must stay untouched.
- Related trap on the same PRs: a wall-clock performance assertion (`expect(Date.now() - started).toBeLessThan(2_000)`) failed at 10,677 ms on a contended runner while the implementation was correctly linear — 561 ms locally for the same input. See the existing "CI is ~25× slower than local" entry. Pick a threshold that separates the algorithmic classes you care about (linear vs quadratic), not one that measures runner load.
