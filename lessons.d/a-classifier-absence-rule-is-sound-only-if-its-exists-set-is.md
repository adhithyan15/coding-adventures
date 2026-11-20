---
category: Security boundaries
---

# A classifier absence rule is sound only if its exists-set is derived from the plan that builds the world and pinned against it

**What went wrong (avoided).** Kiln's classifier ran on the host after a step
that ran in a sealed new root, and asked the HOST's disk whether a path existed.
For paths the sandbox does not contain (`/run/systemd/resolve/stub-resolv.conf`,
the `package.json` above the checkout, `/var/run`) that answer is about a place
the step could not see, so every such probe was a `host-read`. The tempting fix,
"anything outside the checkout is absent", is wrong in exactly the places the plan
mounts something: Home, Tmpdir, scratch tmpfs, declared roots, the ancestors made
to reach them, `/dev`, `/proc`.

**The fix (KILN04 D43).** An explicit exists-set (`traced/sealed.go`), a superset
of what `internal/sandbox/plan.go` mounts, fed by the SAME configuration and the
same `sandbox.ScratchDirs` the plan uses; a test in `internal/record` builds the
plan for four layouts and requires every mount target and every directory on the
way to it to be in the set, so a new mount that the classifier does not know about
fails a test instead of becoming a silent "absent" constant. A real-sandbox test
shows the ENOENTs inside the real thing. A success at a "sandbox-absent" path is
refused, so a wrong set shows up as a reason, never as a false cache hit.

**Found on the way.** A failed probe whose walk stopped at a cwd-text component
the disk lacked, with a `..` after it, was judged by the text alone, so a cwd
directory renamed away (and not re-created) redirected it. Reachable only for a
cwd the trace never walked, which is why the fuzzer could not find it; the unit
tests and the `reshaped` whole-action flag cover it.

**Do differently.** Before turning "ask the disk" into "constant", write the
exists-set from the construction of the world, make it a superset, derive its
inputs from the builder's own variables, pin the two together in a test, and
keep the contradicting case (a success where the model says nothing is) refused.
When a fuzzer cannot reach a defence, say so and test the defence directly.
