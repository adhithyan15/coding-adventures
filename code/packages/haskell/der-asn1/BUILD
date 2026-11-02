# The Darwin GHC bindist links CLT SDK stubs: use the matching CLT linker.
# Scope the selection to Cabal and its children, keeping other packages intact.
if [ "$(uname -s)" = Darwin ]; then DEVELOPER_DIR=/Library/Developer/CommandLineTools cabal test && DEVELOPER_DIR=/Library/Developer/CommandLineTools cabal build der-asn1:lib:der-asn1 der-asn1:test:spec --ghc-options=-Wall --ghc-options=-Werror; else cabal test && cabal build der-asn1:lib:der-asn1 der-asn1:test:spec --ghc-options=-Wall --ghc-options=-Werror; fi
