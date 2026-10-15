## [0.470.0] - UNRELEASED

- Extend bounded sign-rooted `entier` proofs from eight additive offset units to nine, accepting exact `9` and checked combinations while preserving the pathwise nested cap.
- Reconcile using the owned canonical literal-cost proof; retain shared exp/log, legacy, effect and 8192-work guards. The full affected suite passed 1,809 tests over 78 targets with one existing ignore, plus strict lint and independent source/terminal review. Hosted acceptance remains pending; final-source Miri DEFERRED/PENDING.
