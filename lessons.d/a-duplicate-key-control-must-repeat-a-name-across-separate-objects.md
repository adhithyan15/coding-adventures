# A duplicate-key control must repeat a name across separate objects

When testing duplicate JSON members, a positive control must repeat a key in
different nested object scopes (for example, `pairs` in the root and inside
`config`). Repeating the same key inside one object is itself invalid and makes
the supposed control fail for the intended reason. Pair each duplicate
rejection case with a known-valid complete report under current metadata rules.
Run Ruby package tests through the full `rake test` target when SimpleCov's
minimum applies to the package; invoking only the new test file measures too
little of the package and correctly falls below the whole-package threshold.
---
category: Testing & coverage
---
