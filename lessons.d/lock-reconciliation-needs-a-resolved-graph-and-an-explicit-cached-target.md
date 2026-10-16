---
category: Rust
---

# Lock reconciliation needs a resolved graph and an explicit cached target

After the collision version-only merge, metadata --no-deps returned success but
did not resolve/update the ignored geometric0.39 lock entry. The pre-write delta
assertion failed; preserve the unchanged-lock diagnostic. Full offline metadata
then failed fetching uncached android_system_properties0.1.6 before producing JSON;
its receipt preserves actual exit/diagnostic and is not acceptance.

Resolve metadata with --filter-platform set to rustc's actual host when validating
that cached target. Actual Windows-host native and standalone metadata commands
passed, only geometric0.39->0.40 changed, every registry record stayed exact and
the Closure lock stayed byte-exact. This is not other-target execution evidence;
keep conservative all-kind local input coverage separately. Save each command's
raw outcome before parsing or delta assertions so failure evidence survives.
