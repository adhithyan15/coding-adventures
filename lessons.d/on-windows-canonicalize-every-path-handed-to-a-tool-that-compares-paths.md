---
category: Cross-platform & Windows BUILD_windows
---

# On Windows, canonicalize every path handed to a tool that compares paths, or none

**What went wrong.** `warrant-corpus-gen`'s test
`the_oracles_verify_v2_accepts_a_generator_written_corpus` passed `--repo-root` through
Rust's `Path::canonicalize`, but passed `--warrant` and `--profile` as written
(`<crate>/../../../../code/...`). On Windows, `canonicalize` returns a verbatim path,
`\\?\D:\a\...`. The Python oracle `resolve()`s the `.warrant` path to the plain
`D:\a\...` and then calls `relative_to(repo_root)`. The two spellings never share a
prefix, so the check fails with "--warrant is not inside the repository root". On Linux
and macOS there is no verbatim prefix, so the test passed there, and it went red only on
the `build (windows-2025)` job of a later PR that rebuilt the crate.

**The fix.** One helper, `plain()`, canonicalizes each path and strips a leading `\\?\`.
Every path the test hands the oracle goes through it.

**Do differently.** When a test passes paths to another program that compares them
(`relative_to`, `starts_with`, prefix checks), normalize all of them with one function,
or none of them. Never canonicalize only one. On Windows, a bare `canonicalize()` result
is not a path that Python, Node or a shell will compare equal to a plain one.
