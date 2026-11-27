---
category: Testing & coverage
---

# A Warrant model or helper may not be named bit because the checker reserves built-in names

**What went wrong.** The first draft of `code/specs/logic-gates-model.warrant` named its oracle-side Bool-to-Int helper `bit`. The checker refused the document with `W0201 DuplicateSymbol("bit")`: `bit` is a built-in, and a model may not redefine a built-in name. The failure shows only in `warrant-grammar`'s repository-wide `tests/v0_compat.rs`, not in any package BUILD.

**Fix.** Rename the helper (`b2i`). A bit-twiddling helper name is the likeliest collision; check the checker's built-in table before naming one `bit`, `shl`, `shr`, `div`, `len` and the like.

**Do differently.** Run `cargo test -p coding_adventures_warrant_grammar --test v0_compat` on every new `.warrant` before writing the claims, so a name collision surfaces in seconds. Note that `sed -i` with `\b` does not work on macOS (BSD sed): rename with a script, then grep for the old name.
