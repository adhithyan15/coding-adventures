---
category: Repo policy / workflow reminders
---

# Two open PRs can each add a language-specific copy of the same generator; check main for a generic table before adding one

Perl LANG84 4a (PR #17850) and Ruby R0 (PR #17821) were open at the same time,
and each added a `releases.csv`-driven, SHA-256-deduplicating compiled-grammar
generator to `code/programs/rust/grammar-tools/src/lib.rs`. Ruby's was generic
(`RustReleaseFamilySpec`); Perl's was a Perl-only branch of the versioned
families. Ruby merged first, so the Perl PR conflicted in `lib.rs`, `README.md`
and `CHANGELOG.md`, and the two readers had drifted apart on security checks:
Ruby's rejected field-count mismatches and quoted rows, Perl's rejected
duplicate IDs and unknown statuses, and neither had all four.

Git's auto-merge also produced a `Cargo.toml` with the `coding_adventures_sha256`
dependency listed twice, once from each side, which cargo refuses with
`duplicate key`. That file was not reported as conflicted.

The fix was to take main's generic generator, add a `"perl"` row, and extend
the generic path only where Perl needed it (an unversioned token grammar, and
a stage derived from `grammar_status` for an inventory that predates the
`grammar_stage` column), with the union of both readers' row checks. The Perl
consumers moved to main's generated API (`INVENTORY` instead of
`INVENTORY_RELEASES`, plus `grammar_class`).

Before adding a language-specific path to shared tooling, `git fetch` and
check both main and the open PRs that touch the same file. Join an existing
generic table if one exists. After any merge in which both sides added the
same dependency, build once before trusting the auto-merged manifest.
