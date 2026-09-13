# Enum-variant additions: grep the WHOLE repo for matchers, not just the crates you plan to touch (CLOC12.175)

Adding `ClassMember::Field` to javascript-ast broke `javascript-parser`'s **test**
build (`let ClassMember::Method(m) = &c.body[0];` → refutable) — a crate I never
built locally because I only tested the 9 pass crates + emitter I edited. CI's
affected-package graph builds ALL transitive consumers of the changed shared crate,
so it caught it on macos ("Build and test affected packages"). Fix: before pushing
a shared-enum change, `grep -rl --include="*.rs" "EnumName::" code/` to find EVERY
consumer (production AND test code — refutable-let sites live in tests too), and
build each. A per-crate build of only the crates you edited is NOT the affected set.
Cross-refs feedback_run_downstream_consumer_tests, feedback_affected_package_latent_bugs.
