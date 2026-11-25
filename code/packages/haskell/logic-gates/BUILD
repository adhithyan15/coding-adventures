# The Darwin GHC bindist links CLT SDK stubs: use the matching CLT linker.
# -Werror is enforced here (not in the .cabal file) so a published library never carries it.
# Unlike sibling packages, a missing cabal is a FAILURE here, never a silent skip.
if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; if command -v cabal >/dev/null 2>&1; then cabal test --ghc-options=-Werror; else echo 'cabal not found -- cannot test logic-gates' >&2; exit 1; fi
