---
category: Rust
---

# Offline resolution can select unreviewed cached dependency versions

The D1 startup guard refused its freshly resolved standalone lock before tests:
although cargo ran offline, cached syn3.0.9 and unicode-ident1.0.27 differed from
the already accepted native registry cohort. Offline means no download, not
version identity. Save the rejected lock and compare every resolved registry
name/version/source/checksum tuple to the accepted cohort. Use explicit offline
precise updates to restore compatible accepted syn3.0.8/unicode-ident1.0.26;
verify the complete subset before compiling. Do not call a resolver admission
refusal a semantic red. Existing source manifests and native/frozen locks remain
unchanged. Run repository lesson tools from the root or use their absolute path;
a root-relative invocation from the standalone package failed before file writes.
