# The Darwin GHC bindist links CLT SDK stubs: use the matching CLT linker.
# Scope the selection to Cabal and its children, keeping other packages intact.
# `cabal test all` runs both suites: `spec` (units, laziness / async-exception /
# budget-tree hazards, negative controls) and `pbt-v1-conformance` (the shared
# corpus, which must print `pbt-v1: 25/25`).
if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; cabal test all
