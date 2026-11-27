---
category: BUILD files & dependency management
---

# Match repository-input selectors to exact discovered package names

The discovery engine names a package from its language and the directory's
literal basename. A repository-input selector that independently normalizes
that name can miss the package or omit an input from its cache key; Ruby's
`unix_history_simulator` directory is one example. Derive selector identities
from actual discovery results, keep fixture selection and cache-input dispatch
on the same exact name, and use an explicit canonical-name expectation in the
regression test instead of the selector helper as its own oracle.
