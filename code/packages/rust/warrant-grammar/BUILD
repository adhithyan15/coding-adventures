cargo test -p coding_adventures_warrant_grammar
cargo clippy -p coding_adventures_warrant_grammar --all-targets -- -D warnings
cargo run -q -p coding_adventures_warrant_grammar --example regenerate -- --check
cargo test -q -p coding_adventures_warrant_grammar --test self_corpus -- --sabotage always-pass 2>/dev/null | grep -q '^not ok '
python3 ../../../scripts/warrant_syntax_corpus.py
python3 -m unittest discover -s ../../../scripts/tests -p test_warrant_syntax_corpus.py
python3 ../../../scripts/warrant_corpus_oracle.py
python3 -m unittest discover -s ../../../scripts/tests -p test_warrant_corpus_oracle.py
cargo run -q -p coding_adventures_warrant_grammar --example gen_corpus -- --check
python3 ../../../specs/fixtures/warrant-vectors-synthetic-v1/generate.py --check
