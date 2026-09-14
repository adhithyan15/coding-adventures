# Verifying a package by running its tests can miss the check that actually fails CI

**What happened (HL-C242).** Chinese chapter 7 was verified locally with
`npx vitest run` in all three packages that read `human-languages`, plus the
five `check:*` gates. All green. CI then failed on `language-ladder` with:

    bundle check: largest eager chunk is 500087 bytes (limit 500000)

Eighty-seven bytes over, on a chapter that added five lessons.

**Why local verification could never have caught it.** `scripts/check-bundle.mjs`
is not a vitest test. It runs in the package's BUILD, after `vite build`, and
reads `dist/`. No amount of running the test suite reaches it. The script even
guards against a stale `dist/` — it refuses to report if any source is newer
than the built `index.html` — precisely because a measurement that cannot move
reads as evidence.

**Rule:** for a package whose BUILD does more than compile — bundle budgets,
size gates, generated-artifact checks — run the build, not just the tests:

    npm run build && node scripts/check-bundle.mjs
