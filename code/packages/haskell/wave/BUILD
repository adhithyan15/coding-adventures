# The Darwin GHC bindist links CLT SDK stubs: use the matching CLT linker.
# Scope the selection to Cabal and its children, keeping other packages intact.
if [ "$(uname -s)" = Darwin ]; then DEVELOPER_DIR=/Library/Developer/CommandLineTools cabal test all; else cabal test all; fi
