---
category: Repo policy / workflow reminders
---

# A trivial fix for a failure outside the PR goes into the PR the same hour, not into a wait for main

CI installs Rust with `dtolnay/rust-toolchain ... # stable`, so the toolchain
floats. When stable moved to 1.99, its clippy gained a deny-by-default
`approx_constant` case for `f64::consts::EULER_GAMMA`. That broke
`statistics-core`'s `digamma_known_values` test (a hand-typed `0.5772156649`)
in every run that built the crate, including #16391, which never touched it.

I commented the two-line patch on the PR and queued a separate task. Then I
waited more than seven hours, re-checking every hour, for main to pick it up.
The PR sat red on both platforms, and the emulator gate it needed never ran.
The owner had to tell me to unblock myself.

**Fix:** port the two lines into the PR (`std::f64::consts::EULER_GAMMA`,
already stable on 1.94 too). Once main carries the same fix, the change has
no effect.

**Do instead:** when a failure outside the PR blocks it, the fix is small and
obviously correct, and no fix exists elsewhere, port it into the PR right
away. Review it, push, and note it in the PR body. "Keep the PR scoped" is
not a reason to leave it red. A comment plus waiting is only for fixes that
are large, risky, or need an owner's decision. When CI's floating `stable`
breaks an untouched crate, check with `cargo +stable clippy --all-targets`
locally (`rustup toolchain install stable`) before deciding.
