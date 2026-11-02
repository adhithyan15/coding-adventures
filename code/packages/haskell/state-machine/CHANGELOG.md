# state-machine changelog

- Unreleased: `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.
- `0.1.0`: initial Haskell port covering DFA, NFA, PDA, minimization, and modal state machines.
