# CV04a native CI portability repair

CI run 38169475256 on candidate a46ca7ffb2087a80f55a1bd4f4b3cbe8ebfbf27d
completed Windows Closurec but failed two unchanged Chief regressions. Linux's
hosted runner lost communication, leaving its package step incomplete. These
are distinct failures; neither a partial compiler row nor a rerun substitutes
for terminal acceptable required checks and three-platform native acceptance.

## Resolver boundary

`SystemResolver::resolve("", port)` must return `InvalidInput` before invoking
the platform resolver. Windows accepts an empty name as a local-host lookup;
the public resolver boundary must not depend on that platform interpretation.
Nonempty host behavior and the existing all-address public-network checks stay
unchanged. Tests require the exact error kind for an empty host, preserve the
localhost host-table assertions, and do not use external DNS.

## Killed child observation

A never-ready child must produce `BootstrapTimeout`, be killed and reaped,
enter `Exited`, and have no process ID. The existing 500 ms production bootstrap
limit, 2 s grace bound, and 5 s test observation bound stay unchanged. Its exact
exit status follows the OS: Unix signal termination has no numeric code;
Windows `Child::kill` uses termination code 1. Assert those exact platform
values, rather than waiting for Unix's `None` on Windows or accepting arbitrary
exit status. Supervisor production behavior and containment remain unchanged.

## Delivery evidence

Commit this specification before tests and implementation. Reproduce the
Windows failures, run the complete repaired packages and affected reverse
dependencies with strict lint, retain Closurec/source payload identity, and
obtain independent security review on the final candidate before pushing to
the existing PR. Fresh required CI must execute unchanged assertions on all
platforms. Do not skip tests, increase limits, bypass required checks, or claim
the lost Linux runner proved package success.

Full local Miri remains **DEFERRED / PENDING**, never passed. Preserve the
original frozen checkout/run/497 inputs and receipts. The Closure compiler,
full lineage/parity contract, and final-source Miri objective remain incomplete.
