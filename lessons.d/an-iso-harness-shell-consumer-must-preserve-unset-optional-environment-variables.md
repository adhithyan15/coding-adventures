---
category: Testing & coverage
---

# An iso-harness shell consumer must preserve unset optional environment variables

**What went wrong.** A new C package test script used `set -u` before sourcing
`iso-harness/lib/iso-lib.sh`. The shared harness intentionally reads optional
`ISO_REQUIRE` when callers have not set it, so nounset stopped the test before
compiler discovery.

**Fix.** Keep `set -e` for command failures but leave optional harness
environment variables unset when not configured. Set `ISO_REQUIRE` explicitly
only in CI lanes that require particular compilers.

**Next time.** Match the existing C package runner pattern before adding shell
strict-mode options around shared helpers; a helper's optional environment
contract is part of its interface.
