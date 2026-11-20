The shared Rust source-tree framer now rejects a symlinked or reparse-point
root and applies aggregate file, entry and byte caps while collecting source.
Ordinary accepted trees keep their existing sorted `warrant-src-v1` bytes.
