cargo test -p coding_adventures_zeroize -- --nocapture
# VER04k section 4 (WX-G): the pilot evidence harness is generated from
# code/specs/zeroize.warrant and evidence.bindings.  Fail if the checked-in file
# drifted, run it (every `ensures` row monitors every call, every claim runs over
# its labelled domain), and prove each sabotage mode makes a claim report `not ok`:
# a mutation lost on a copy, an off-by-one last write, a store of the wrong value.
cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check zeroize
cargo test -q -p coding_adventures_zeroize --test warrant_evidence
WARRANT_EVIDENCE_FAULT=stale-post cargo test -q -p coding_adventures_zeroize --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=skip-last-write cargo test -q -p coding_adventures_zeroize --test warrant_evidence 2>/dev/null | grep -q '^not ok '
WARRANT_EVIDENCE_FAULT=cursor-off-by-one cargo test -q -p coding_adventures_zeroize --test warrant_evidence 2>/dev/null | grep -q '^not ok '
