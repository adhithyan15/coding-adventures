cargo test -q -p lzw --lib -- --nocapture
cargo test -q -p lzw --doc
# Hand-written integration tests (SEC-001/SEC-002): hostile headers, the 16 MiB output cap,
# linear dictionary memory, linear encoder time, and the old encoder and decoder kept as
# references.  Named so a bare `cargo test -p lzw` does not also run the generated evidence
# target below, which takes minutes.
cargo test -q -p lzw --test hostile_headers --test decompression_cap --test decoder_memory --test encoder_cost --test decoder_equivalence
# The library is pure: no capability, test code included (the analyzer reads it).
cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .
# VER08b: compress and decompress, including hostile input and the output cap, run inside a
# seccomp/Landlock jail derived from required_capabilities.json (no capabilities),
# watched by an independent strace; on macOS this plain run reports the gate NOT-LIVE (RCG010) without failing
# BUILD_linux runs the same line with RCG_STRICT=1 so a NOT-LIVE gate is a failure there.
# (Windows uses BUILD_windows.)  Everything else in BUILD_linux is identical to this file.
cargo test -q -p lzw --test runtime_capability
# VER04h: the evidence harness is generated from code/specs/lzw.warrant and
# evidence.bindings.  Fail if the checked-in file drifted, rewrite the evidence manifest
# from a real run (about nine minutes: every instance runs under the allocation observer),
# and prove each sabotage mode makes a model-pinned exhaustive suite report `not ok` (each
# filter names the cheap claim that pins the faulted behaviour; a fault applies to every
# operation that lists it, and the filter selects which operation's claim is attacked).
cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check lzw
WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p lzw --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/compress_one_byte_matches_the_model cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=drop-last-result-byte WARRANT_EVIDENCE_FILTER=/exhaustive/compress_one_byte_matches_the_model cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/compress_two_bytes_match_the_model cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=drop-last-result-byte WARRANT_EVIDENCE_FILTER=/exhaustive/compress_two_bytes_match_the_model cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=drop-last-result-byte WARRANT_EVIDENCE_FILTER=/exhaustive/compress_is_at_least_seven_bytes cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/round_trip cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=drop-last-result-byte WARRANT_EVIDENCE_FILTER=/exhaustive/round_trip cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/round_trip_of_long_runs cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=drop-last-result-byte WARRANT_EVIDENCE_FILTER=/exhaustive/round_trip_of_repeated_byte_pairs cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/declared_length_cuts_the_output cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=drop-last-result-byte WARRANT_EVIDENCE_FILTER=/exhaustive/a_huge_declared_length_is_harmless cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decompress_empty_input_rejected cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decompress_one_byte_rejected cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decompress_header_only_rejected cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decompress_one_payload_byte_rejected cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decompress_two_byte_payload_model cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decompress_missing_clear_rejected cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decompress_invalid_first_code_rejected cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decompress_invalid_first_code_every_value cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decompress_two_bytes_rejected cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decompress_three_bytes_rejected cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/decompress_zero_header_two_byte_payload_model cargo test -q -p lzw --test warrant_evidence 2>/dev/null | grep -q '^not ok '
# No operand is secret, so there is no constant-time obligation; the steps rows are
# attempted on the real source, the front end refuses it (the `'a` of BitReader), and the
# proof manifest records every steps-bound obligation `not-live` with the blocker.
WARRANT_PROOF_OUT=$PWD/evidence cargo test -q -p coding_adventures_warrant_ir --test lzw_proofs
# VER04h: the report is derived from the manifests written above; --check cross-checks it
# against them and fails closed on any disagreement, CWE-409 included (a decompressor is
# never excluded: the row is evidenced by the output/heap bound rows or the check fails).
# --allow-unverified: the report still cites `unverified::` static attestations (AST audits
# no tool re-runs, same rows as the other adopters).  --profile pins the audit profile: the
# report records the one it was generated under and --check refuses a different one, so a
# decompressor cannot be re-labelled (and its CWE-409 row excluded) by regenerating it.
cargo run -p coding_adventures_ca_assurance_report -- --check assurance_report.json --allow-unverified --profile lossless-compression
