## [0.1.0] - UNRELEASED

### Added (a Go-only recording selector, KILN04 7.3)

- `kiln record prepare --language <name|all>` (default `all`). A name passes the build tool
  `-plan-closed-language <name>` instead of `-language all` (never both), so the shard plan holds
  only that language's prerequisite-closed packages. `PlanArgs` gains a `language` parameter; for
  `all` its output is byte-identical to before. The graph and steps exports stay whole.
- `binding.language` (`omitempty`): the selection, sealed beside the plan hash. A default binding
  keeps its exact bytes and id (a test reproduces the pre-field encoding). `"all"` is not a legal
  value, so a selection has one spelling and one id. `recfmt.CheckLanguage`.
- `record-merge` reads the selection only from the shards' bindings (`expectedFor`,
  `selectionOf`), never from a flag or a shard's package list. Shards whose bindings disagree void
  the run (B3), as does a plan package of another language. With a selection, steps entries for
  packages the plan does not run are ignored; with `all` they are still refused.
- New fail-closed rule M16: an absent or untracked observation inside an unselected package's
  directory (nearest enclosing graph package) costs that action its record, via the fixed reason
  `foreign-package-read`. It is not an order dependence and does not void the run. Reading a foreign
  package's tracked sources is unaffected. The merge report prints a count of dropped actions.
- Tests: default binding bytes and ids, `PlanArgs` for both selections, disagreeing bindings, plan
  outside the selection, foreign absent/untracked/tracked reads, nearest-enclosing-package
  nesting, a shard claiming an unselected package (mutant killer), and a selection dimension in
  `FuzzMerge`'s oracle.
- `kiln:recorder` is unchanged: no action's verdict changes.
