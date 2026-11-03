# The Darwin GHC bindist links CLT SDK stubs: use the matching CLT linker.
# Scope the selection to Cabal and its children, keeping other packages intact.
if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; if command -v cabal >/dev/null 2>&1; then cabal test all --builddir=dist; else echo 'cabal not found -- skipping'; fi
