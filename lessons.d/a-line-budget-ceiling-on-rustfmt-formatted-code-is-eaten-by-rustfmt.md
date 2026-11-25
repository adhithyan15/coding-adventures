---
category: Rust
---

# A line-budget ceiling on rustfmt-formatted code is eaten by rustfmt's width heuristics, not by logic

The PIF reader (VER32 PIF-002) has a hard 600 non-blank, non-comment line budget in
`tests/ceiling.rs`. The first complete version, with every rule checked and cited, was 756
counted lines, and almost all of the excess was formatting, not logic: rustfmt splits a
method chain longer than 60 characters (`chain_width`), a struct literal with more than
about 18 characters of fields (`struct_lit_width`), an `if`/`else` expression longer than 50
characters, and any call that passes 100 columns. A check written as
`need(cond, E::X)?;` became four lines each time; about forty checks cost 120 lines.

Fix, without a per-crate `rustfmt.toml` (which would reformat the other kernel files and
conflict with concurrent slices): write each check as `need! { cond, E::X }` with a
brace-delimited macro, which rustfmt leaves alone and which keeps one rule on one line (and
builds the error only on failure); put `#[rustfmt::skip]` on data declarations (error enums,
limit and report structs) and on the few statements whose struct literals would otherwise
split; keep method chains under 60 characters. The count went from 756 to 584.

Rule: when a trusted module has a line ceiling, measure after `cargo fmt`, never before,
and design the shape of the code (one check per line) so the formatter cannot multiply it.
Do not "fix" a budget overrun by dropping a rule: list what is deferred in the README
instead.
