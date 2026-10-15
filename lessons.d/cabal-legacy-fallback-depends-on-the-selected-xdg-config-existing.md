---
category: Testing & coverage
---

# Cabal legacy fallback depends on the selected XDG config existing

The first portable macOS bootstrap fixture expected an explicitly named but absent XDG config to override an existing legacy ~/.cabal directory. Cabal3.10 documents the fallback when CABAL_DIR is unset and the selected XDG config does not exist, including an explicit XDG_CONFIG_HOME. Keep that production rule; test both absent and present custom XDG files. Preserve the actual five-pass/one-fail receipt. This is a config-discovery oracle correction, not evidence of actual macOS linking.

The first workflow edit anchored on an OCaml setup label that appears in two jobs; its uniqueness assertion stopped before modifying the workflow. Anchor the Haskell-only step on the unique package-index verification block so the other job stays exact.
