---
category: Testing & coverage
---

# A test that hard-codes counts from a shared canonical spec file breaks when another PR extends that file, and a fixed-seed rejection sampler with a low retry cap fails deterministically

Two TypeScript packages (`warrant-compiler` #19042, `test-bootstrap` #19039) landed
tests asserting that `code/specs/ct-compare.warrant` has 1 claim and produces 13
results. #19054 then added 8 claims to that file. Each PR was green on its own;
together they turned 2 of the 15 `main` full-build shards red.

Two separate defects hid behind the one red check:

- **Pinned counts.** `claims.length === 1` and `results.length === 13` described the
  file as it was when the test was written. Updating them to 9 and 21 fixed the
  first assertion only.
- **A deterministic "flake" under the new claims.** `select_picks` filters on
  `len(a) == len(b)` over two independent `Bytes` binders (length 0..32), accepted
  about 1 time in 33. The generator's rejection-sampling cap was 100 attempts, so
  roughly 5% of samples exhausted it ("Generator filter exhausted"). With a fixed
  seed that is not a 5% flake: the same sample fails every run. The cap is now 1000.

What to do differently:

- When a test pins a count taken from a shared spec or corpus file, derive the
  number from the file (or put the expected value next to a comment naming the
  file and the PR that last changed it), and grep for *other* tests that read the
  same file before landing a change to it. Here the same test existed in two
  packages.
- After fixing the assertion a failure first points at, re-run the whole suite:
  the next failure was behind it and only showed once the first stopped masking it.
- Size a rejection-sampling cap from the acceptance rate, not from habit; a fixed
  seed turns "rarely fails" into "always fails" for one particular sample.
