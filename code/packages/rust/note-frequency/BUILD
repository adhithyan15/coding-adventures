# needs-toolchain: python
cargo clippy --locked -p coding-adventures-note-frequency --all-targets -- -D warnings
cargo test --locked -q -p coding-adventures-note-frequency --lib -- --nocapture
cargo test --locked -q -p coding-adventures-note-frequency --doc
cargo test --locked -q -p coding-adventures-note-frequency --test note_frequency_tests --test test_hostile_inputs --test test_pbt
cargo test --locked -q -p coding-adventures-note-frequency --release --test warrant_evidence
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir .
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir . --check-module
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir . --verify-cert effect_certificate.json
cargo run --locked -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check note-frequency
WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test --locked -q -p coding-adventures-note-frequency --test warrant_evidence
WARRANT_EVIDENCE_FAULT=field-flip WARRANT_EVIDENCE_FILTER=/exhaustive/parse_note_parts_valid_two_bytes_matches_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding-adventures-note-frequency --test warrant_evidence
WARRANT_EVIDENCE_FAULT=field-flip-last WARRANT_EVIDENCE_FILTER=/exhaustive/parse_note_parts_valid_two_bytes_matches_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding-adventures-note-frequency --test warrant_evidence
WARRANT_EVIDENCE_FAULT=ok-as-err WARRANT_EVIDENCE_FILTER=/exhaustive/parse_note_parts_valid_two_bytes_matches_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding-adventures-note-frequency --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/parse_note_parts_one_byte_rejected python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding-adventures-note-frequency --test warrant_evidence
WARRANT_EVIDENCE_FAULT=field-flip WARRANT_EVIDENCE_FILTER=/exhaustive/semitones_from_a4_parts_all_bytes_oct4 python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding-adventures-note-frequency --test warrant_evidence
WARRANT_EVIDENCE_FAULT=ok-as-err WARRANT_EVIDENCE_FILTER=/exhaustive/semitones_from_a4_parts_all_bytes_oct4 python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding-adventures-note-frequency --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/semitones_from_a4_parts_rejects_out_of_range_octave python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding-adventures-note-frequency --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/note_to_frequency_millihertz_a_octaves python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding-adventures-note-frequency --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/note_to_frequency_millihertz_one_byte_rejected python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p coding-adventures-note-frequency --test warrant_evidence
cargo run --locked -p coding_adventures_ca_assurance_report -- --check assurance_report.json --allow-unverified --profile note-frequency --profile-kind transform
