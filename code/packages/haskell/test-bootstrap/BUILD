# The Darwin GHC bindist links CLT SDK stubs: use the matching CLT linker.
# Scope the selection to Cabal and its children, keeping other packages intact.
# `cabal test all` runs every suite: `spec` (units, laziness / async-exception /
# budget-tree hazards, execution isolation, negative controls), the shared-corpus
# `pbt-v1-conformance` (must print `pbt-v1: 25/25`) and `fuzz-v1-conformance` (must
# print `fuzz-v1: 25/25` and `vectors: 820/820`).
if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; cabal test all
