---
category: Supply chain & CI pinning
---

# Pin to the version that FIXES the bug you are citing, not the one you happen to have installed

Pinning `cargo-tarpaulin` to the locally installed 0.35.2 would have frozen CI on the last release *before* upstream fixed the very SIGILL the pin was written to prevent (fixed in 0.35.3, yanked, republished as 0.35.4). Check the upstream changelog and crates.io before pinning; that check is also what surfaces yanks.
