# The L0 bootstrap harness (TF01). One command per line: the build tool runs
# each line as its own `sh -c`, so no backslash continuations.
#
# 1. Unit tests, then the self-corpus run, which must exit 0.
cargo test -p coding_adventures_test_bootstrap
cargo clippy -p coding_adventures_test_bootstrap --all-targets -- -D warnings
# 2. Sabotage (TF01 §6): with every core verdict forced to Pass, the corpus's
#    expected-fail cases must be reported as `not ok`. grep checks that from
#    OUTSIDE the harness, so a harness that could never report failure fails
#    here. The pipeline's status is grep's: no `not ok` line, no pass.
cargo test -q -p coding_adventures_test_bootstrap --test self_corpus -- --sabotage always-pass 2>/dev/null | grep -q '^not ok '
# 3. The independent Python reference agrees with cases.json, and cases.tfv
#    is exactly what cases.json generates (for both TF01 and VER05 PBT).
python3 ../../../scripts/bootstrap_harness_conformance.py
python3 ../../../scripts/pbt_conformance.py
# 4. Differential (TF01 section 8.1): the Rust parser's verdict on every
#    bounded-exhaustive TFV document, and its deep-equality matrix, are replayed
#    through the independent Python reference. Any disagreement fails the build.
TFV_DIFFERENTIAL_OUT=../target/tfv-differential.tsv cargo test -q -p coding_adventures_test_bootstrap --test tfv_exhaustive
python3 ../../../scripts/tfv_differential.py ../target/tfv-differential.tsv
python3 ../../../scripts/tfv_differential.py --self-test
