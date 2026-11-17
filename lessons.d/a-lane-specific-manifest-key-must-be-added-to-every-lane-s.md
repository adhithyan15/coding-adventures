---
category: CI & GitHub Actions
---

# A lane-specific manifest key must be added to every lane's warrant-corpus validator in the same PR, because each lane validates the others' manifests

WD-004b2 made the Python lane record `pycache_isolated: true` in its `ct-compare` manifest and taught only the Python validator and the Rust loader about it. CI then failed in Go's `TestRealLaneManifestsFromOtherLanesAreValidated` with "manifest has unexpected fields": every lane's validator checks a closed key set and also validates the committed manifests of the other lanes (WD-102), so a key one lane emits is an unknown field to any validator that was not updated. The Python validator had the opposite bug, requiring the key on every manifest, which would have rejected the Go and TypeScript ones.

The fix was to port the VER04i 8.1 rule into all three validators: the key is required, and exactly `true`, when `lane == "python"`, and forbidden for every other lane. The lane must be read before the closed key check, because the permitted key set depends on it. Each validator's change also alters the runner source digest, so each lane's `ct-compare` manifest and label sidecar had to be regenerated with that lane's own `run --out` command.

Next time: when a manifest gains a key that only one lane writes, change the Go, Python, TypeScript and Rust validators in the same PR, add a refusal test for each direction (missing where required, present where forbidden), and run each lane's `ct-compare` `check` and `validate-manifest` steps locally before pushing.
