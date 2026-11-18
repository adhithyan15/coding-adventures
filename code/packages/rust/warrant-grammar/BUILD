# needs-toolchain: python
# WD-HARN1c: the shared negative-control checker (code/scripts/warrant_negctl.py) is called by
# adopter BUILD lines; its tests run here so editing the script reruns them (see repository_inputs.go).
cargo test -p coding_adventures_warrant_grammar
cargo clippy -p coding_adventures_warrant_grammar --all-targets -- -D warnings
cargo run -q -p coding_adventures_warrant_grammar --example regenerate -- --check
python3 ../../../scripts/warrant_negctl.py -- cargo test -q -p coding_adventures_warrant_grammar --test self_corpus -- --sabotage always-pass
python3 ../../../scripts/warrant_syntax_corpus.py
python3 -m unittest discover -s ../../../scripts/tests -p test_warrant_syntax_corpus.py
python3 ../../../scripts/warrant_corpus_oracle.py
python3 -m unittest discover -s ../../../scripts/tests -p test_warrant_corpus_oracle.py
python3 -m unittest discover -s ../../../scripts/tests -p test_warrant_negctl.py
python3 -m unittest discover -s ../../../scripts/tests -p test_ver04p_domain_witnesses.py
cargo run -q -p coding_adventures_warrant_grammar --example gen_corpus -- --check
python3 ../../../specs/fixtures/warrant-vectors-synthetic-v1/generate.py --check
