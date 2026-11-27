# OS architecture probe fixtures

`profiles.json` is portable input and expected output for the HOS03 semantic
engine probe. Each case names a policy profile, external stimuli, run budgets,
the expected logical tick and profile state, and the ordered data-only effects.
The engine adds generic clock ticks when the input queue is empty. The package
unit suite uses inline stimuli; `verify_profiles.mjs` reads this canonical JSON
fixture during the package BUILD and checks the implementation without granting
the reusable package host filesystem access. These synthetic profiles are
contract examples, not historical or production OS traces.
