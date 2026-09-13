# Case-variant path tests need distinct parents on case-insensitive filesystems

A Rust discovery regression initially created `_build` and `_Build` beneath the same parent; on Windows they named one directory, so the exact-name skip appeared to suppress the preserved case variant. Put exact, case-variant, and near-name fixtures beneath distinct parent components, then assert both the skipped decoy and the retained source packages. This tests the component rule instead of the host filesystem's casing semantics.
