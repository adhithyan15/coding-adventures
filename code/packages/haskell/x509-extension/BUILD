# The Darwin GHC bindist links CLT SDK stubs: use the matching CLT linker.
# Scope the selection to Cabal and its children, keeping other packages intact.
if [ "$(uname -s)" = Darwin ]; then DEVELOPER_DIR=/Library/Developer/CommandLineTools cabal test && DEVELOPER_DIR=/Library/Developer/CommandLineTools cabal build x509-extension:lib:x509-extension x509-extension:test:spec --ghc-options=-Wall --ghc-options=-Werror; else cabal test && cabal build x509-extension:lib:x509-extension x509-extension:test:spec --ghc-options=-Wall --ghc-options=-Werror; fi
