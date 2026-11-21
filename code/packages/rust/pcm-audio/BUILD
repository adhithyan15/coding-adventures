# needs-toolchain: python
cargo clippy --locked -p pcm-audio --all-targets -- -D warnings
cargo test --locked -q -p pcm-audio --lib -- --nocapture
cargo test --locked -q -p pcm-audio --doc
cargo test --locked -q -p pcm-audio --test pcm_audio_tests --test wav_tests
cargo test --locked -q -p pcm-audio --release --test warrant_evidence
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir .
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir . --check-module
cargo run --locked -q -p coding_adventures_ca_capability_analyzer -- --dir . --verify-cert effect_certificate.json
cargo run --locked -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check pcm-audio
WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test --locked -q -p pcm-audio --test warrant_evidence
WARRANT_EVIDENCE_FAULT=narrow-width WARRANT_EVIDENCE_FILTER=/exhaustive/clamp_pcm16_agrees python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p pcm-audio --test warrant_evidence
WARRANT_EVIDENCE_FAULT=sign-confuse WARRANT_EVIDENCE_FILTER=/exhaustive/clamp_pcm16_agrees python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p pcm-audio --test warrant_evidence
WARRANT_EVIDENCE_FAULT=field-flip WARRANT_EVIDENCE_FILTER=/exhaustive/data_size_mono_16_matches_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p pcm-audio --test warrant_evidence
WARRANT_EVIDENCE_FAULT=ok-as-err WARRANT_EVIDENCE_FILTER=/exhaustive/data_size_mono_16_matches_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p pcm-audio --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/data_size_mono_16_matches_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p pcm-audio --test warrant_evidence
WARRANT_EVIDENCE_FAULT=field-flip WARRANT_EVIDENCE_FILTER=/exhaustive/wav_file_size_matches_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p pcm-audio --test warrant_evidence
WARRANT_EVIDENCE_FAULT=ok-as-err WARRANT_EVIDENCE_FILTER=/exhaustive/wav_file_size_matches_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p pcm-audio --test warrant_evidence
WARRANT_EVIDENCE_FAULT=missing-error WARRANT_EVIDENCE_FILTER=/exhaustive/wav_file_size_matches_model python3 ../../../scripts/warrant_negctl.py -- cargo test --locked -q -p pcm-audio --test warrant_evidence
cargo run --locked -p coding_adventures_ca_assurance_report -- --check assurance_report.json --allow-unverified --profile pcm-audio --profile-kind transform
