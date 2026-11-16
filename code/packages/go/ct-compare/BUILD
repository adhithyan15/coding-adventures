GOWORK=off go test ./... -v -cover
GOWORK=off go run ./cmd/warrant-evidence check evidence/test-manifest.json
GOWORK=off go run ./cmd/warrant-evidence sabotage
GOWORK=off go run ./cmd/warrant-evidence validate-manifest evidence/test-manifest.json --registry ../../../specs/fixtures/ct-compare-warrant-v1/consumers.json
GOWORK=off WARRANT_EVIDENCE_FAULT=ignore-last-byte:eq WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
GOWORK=off WARRANT_EVIDENCE_FAULT=invert:eq WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
GOWORK=off WARRANT_EVIDENCE_FAULT=ignore-last-byte:eq_fixed WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
GOWORK=off WARRANT_EVIDENCE_FAULT=invert:eq_fixed WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
GOWORK=off WARRANT_EVIDENCE_FAULT=ignore-high-bit:eq_u64 WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
GOWORK=off WARRANT_EVIDENCE_FAULT=invert:eq_u64 WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
GOWORK=off WARRANT_EVIDENCE_FAULT=missing-error:select_bytes WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
GOWORK=off WARRANT_EVIDENCE_FAULT=swap-choice:select_bytes WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
# needs-toolchain: rust
# WD-004b2 (VER04i 8.3): the Rust loader re-checks the lane (digests, source freshness, labels);
# --require-active makes an absent, planned or unloadable lane a build failure.
cargo run --quiet --manifest-path ../../rust/ca-assurance-report/Cargo.toml -- --check-lane go/ct-compare --repo-root ../../../.. --require-active
