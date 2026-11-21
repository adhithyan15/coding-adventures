# needs-toolchain: python
cargo clippy --locked -p audio-device-sink --all-targets -- -D warnings
cargo test --locked -q -p audio-device-sink --lib -- --nocapture
cargo test --locked -q -p audio-device-sink --doc
cargo test --locked -q -p audio-device-sink --test audio_device_sink_tests --test test_hostile_inputs --test test_pbt
cargo test --locked -q -p audio-device-sink --release --test warrant_evidence
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir .
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir . --check-module
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir . --verify-cert effect_certificate.json
cargo run --locked -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check audio-device-sink
WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test --locked -q -p audio-device-sink --test warrant_evidence
WARRANT_EVIDENCE_FAULT=field-flip WARRANT_EVIDENCE_FILTER=/exhaustive/validate_format_mono_16_all_channels python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p audio-device-sink --test warrant_evidence
WARRANT_EVIDENCE_FAULT=field-flip-last WARRANT_EVIDENCE_FILTER=/exhaustive/validate_format_mono_16_all_channels python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p audio-device-sink --test warrant_evidence
WARRANT_EVIDENCE_FAULT=ok-as-err WARRANT_EVIDENCE_FILTER=/exhaustive/validate_format_mono_16_all_channels python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p audio-device-sink --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/validate_format_mono_16_all_channels python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p audio-device-sink --test warrant_evidence
WARRANT_EVIDENCE_FAULT=field-flip WARRANT_EVIDENCE_FILTER=/exhaustive/frame_count_mono_all_channels python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p audio-device-sink --test warrant_evidence
WARRANT_EVIDENCE_FAULT=ok-as-err WARRANT_EVIDENCE_FILTER=/exhaustive/frame_count_mono_all_channels python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p audio-device-sink --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/frame_count_mono_all_channels python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p audio-device-sink --test warrant_evidence
WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/pcm_byte_len_valid_44100_matches_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p audio-device-sink --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/pcm_byte_len_valid_44100_matches_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p audio-device-sink --test warrant_evidence
cargo run --locked -p coding_adventures_ca_assurance_report -- --check assurance_report.json --allow-unverified --profile audio-device-sink --profile-kind transform
