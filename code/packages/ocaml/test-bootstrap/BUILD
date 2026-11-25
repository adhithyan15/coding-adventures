opam install . --deps-only --with-test --with-dev-setup -y
{ printf 'let json =\n  {pbt|'; cat ../../../specs/fixtures/pbt-v1/cases.json; printf '|pbt}\n'; } | cmp - test/corpus_data.ml
{ printf 'let json =\n  {fuzz|'; cat ../../../specs/fixtures/fuzz-v1/cases.json; printf '|fuzz}\n'; } | cmp - test/fuzz_corpus_data.ml
{ printf 'let table =\n  {govec|'; cat ../go/test-bootstrap/fuzz/reference_vectors_test.go; printf '|govec}\n'; } | cmp - test/go_vectors_data.ml
opam exec -- dune build @fmt
BISECT_FILE="$PWD/bisect" opam exec -- dune runtest --force --instrument-with bisect_ppx
opam exec -- bisect-ppx-report summary --per-file --expect src/bigint.ml --expect src/budget.ml --expect src/canon.ml --expect src/corpus.ml --expect src/error.ml --expect src/escape.ml --expect src/fuzz.ml --expect src/fuzz_bitmap.ml --expect src/fuzz_corpus.ml --expect src/fuzz_engine.ml --expect src/fuzz_executor.ml --expect src/fuzz_failure_ids.ml --expect src/fuzz_handle.ml --expect src/fuzz_handle_internal.ml --expect src/fuzz_hooks.ml --expect src/fuzz_limits.ml --expect src/fuzz_mutate.ml --expect src/fuzz_signature.ml --expect src/fuzz_types.ml --expect src/gen.ml --expect src/prng.ml --expect src/runner.ml --expect src/safe.ml --expect src/shrink.ml --expect src/strict_json.ml --expect src/tfv.ml --expect src/types.ml --expect src/values.ml bisect*.coverage
