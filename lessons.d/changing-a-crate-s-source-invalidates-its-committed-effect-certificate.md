---
category: CI & GitHub Actions
---

# Changing a crate's source invalidates its committed effect certificate

PR #19054 changed `ca-assurance-report`'s `src/` (an evidence-derived report
and a hardened checker). Every local test, clippy and the `ct-compare` BUILD
passed, but all three CI build jobs failed on one line of
`ca-assurance-report/BUILD`:

```
--verify-cert effect_certificate.json
FAIL: Certificate source_digest mismatch
```

The committed effect certificate records a digest of the crate's sources, so
any source edit makes it stale. The authoring agents ran the BUILD of the crate
they were focused on (`ct-compare`) but not the BUILD of every crate they
touched.

Fix: regenerate with
`cargo run -p coding_adventures_ca_capability_analyzer -- --dir . --emit-cert effect_certificate.json`
from the crate directory, then rerun that crate's whole BUILD.

Do differently: before pushing, list every changed crate
(`git diff --name-only origin/main...HEAD -- code/packages/rust | cut -d/ -f4 | sort -u`)
and run each one's BUILD file line by line, not just `cargo test`. Any crate
with an `effect_certificate.json` needs `--verify-cert` to pass after the edit.
