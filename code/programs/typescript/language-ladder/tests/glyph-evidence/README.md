# Language Ladder glyph evidence

This directory is the edit boundary for script-specific Language Ladder tests.
Each `*.evidence.ts` module owns one script or one genuinely shared inventory
relationship. `../independentvowels.test.ts` discovers every module eagerly and
passes one shared corpus/helper context to its cases, so evidence additions do
not edit a registry or reload the application corpus.

For a glyph change:

1. edit only the owning `*.evidence.ts` module;
2. give a new case a nearby numeric `caseOrder` within its suite (ties are
   allowed and receive stable module-path/name tie-breakers);
3. add one unique level-2 entry file under `../../CHANGELOG.d/`; and
4. run `npm run typecheck` and `npx vitest run tests/independentvowels.test.ts
   tests/glyph-evidence-ownership.test.ts`.

Do not add concrete script lookups or assertions to the aggregator. Do not add
an evidence module to a hand-maintained registry: the `import.meta.glob` is the
registry. Cross-script claims belong in a shared module only when the
relationship itself is what the test proves.
