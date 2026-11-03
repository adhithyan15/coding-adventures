---
category: Compiler / VM / language pipeline
---

# A backend feature keyed on module metadata does nothing until the frontend sets that metadata, so check what the frontend emits before fixing a backend

#18798 (item 11) reported that `puts true` printed `#t` on all six SIR
backends.  Every backend already had a Ruby display convention (`true`, an
empty `nil`), chosen by `Module.metadata.source_language == "ruby"`
(`code/specs/sir-display-convention.md`), and every backend had tests that
proved it, built from hand-made modules that set the field.  But
`ruby-to-semantic-ir` built its module with `Metadata::new()`, so no Ruby
program ever reached the Ruby branch.  The fix was one line in the
frontend; patching six display functions would have been wrong.

Do this: when a backend "ignores" a feature that it gates on module
metadata (`source_language`, a manifest feature, a version), dump the
module the real frontend emits (`{:#?}` of `module.metadata`) before
touching a backend.  And give such a feature at least one end-to-end test
that starts from source text, not a hand-built module — the hand-built
tests here all set the field the frontend forgot.
