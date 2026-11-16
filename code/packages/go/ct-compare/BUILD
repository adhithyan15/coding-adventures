go test ./... -v -cover
go run ./cmd/warrant-evidence check evidence/test-manifest.json
go run ./cmd/warrant-evidence sabotage
go run ./cmd/warrant-evidence validate-manifest evidence/test-manifest.json
WARRANT_EVIDENCE_FAULT=ignore-last-byte:eq WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=invert:eq WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=ignore-last-byte:eq_fixed WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=invert:eq_fixed WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=ignore-high-bit:eq_u64 WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=invert:eq_u64 WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error:select_bytes WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=swap-choice:select_bytes WARRANT_EVIDENCE_FILTER=/exhaustive/ go run ./cmd/warrant-evidence run 2>/dev/null | grep -q '^not ok '
