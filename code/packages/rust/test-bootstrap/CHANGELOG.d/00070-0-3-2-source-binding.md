## 0.3.2

- `manifest::read_source_tree`, `manifest::frame_sources` and `manifest::source_tree_framing` (round-2 audit N2): the single definition of the `warrant-src-v1` byte framing (length-prefixed relative path and content of every `src/**/*.rs`, sorted) that producers and the assurance checker hash into `source_sha256`. Symlinks, non-UTF-8 names, oversized files and deep trees are errors.
