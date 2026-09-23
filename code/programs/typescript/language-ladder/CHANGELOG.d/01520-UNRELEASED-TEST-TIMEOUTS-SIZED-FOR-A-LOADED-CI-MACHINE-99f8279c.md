## Unreleased — Test timeouts sized for a loaded CI machine

- Set `testTimeout` and `hookTimeout` to 60 s in `vitest.config.ts`. Vitest's
  defaults (5 s per test, 10 s per hook) assume the suite has a machine to
  itself. This one does not: `beforeAll` in `tests/curriculum.test.ts` awaits
  `loadCurriculumPlans()`, and the ledger plugin transforms the 7,485 lesson
  markdown files under `code/learning/human-languages/*/lessons/`.
- The budgets were measured, not guessed at. On an idle 4-core box the file
  costs 5.0 s and the slowest single test 4.3 s; under 3x CPU oversubscription
  those become 6.4 s and 5.1 s. On a CI runner building the whole repository in
  parallel the hook crossed 10 s and failed, taking all 7 of the file's tests
  down as `skipped` — assertions that never ran, reported as something close to
  a pass. On that same run the slowest individual test finished in 4977 ms,
  clearing the 5 s limit by 23 ms.
- The machine was the variable, not the code: the same commit passed on the
  ubuntu and windows lanes, and the suite's wall clock went from 28–29 s idle
  to 75.99 s there.
- Both budgets are about 12x the idle cost, which is the headroom a loaded
  runner needs. Nothing on this path can hang indefinitely — the hook is a
  single-attempt `Promise.all` over dynamic imports of build-time virtual
  modules, with no network call and no retry loop — and each CI job carries its
  own `timeout-minutes`. Nothing is skipped or disabled: every assertion still
  runs.
