## [0.1.0] - UNRELEASED

### Added (a Go-only recording selector, KILN04 7.3)

- `kiln record prepare --language <name|all>` (default `all`). A name passes the build tool
  `-plan-closed-language <name>` instead of `-language all` (never both), so the shard plan holds
  only that language's prerequisite-closed packages. `PlanArgs` gains a `language` parameter; for
  `all` its output is byte-identical to before. The graph and steps exports stay whole.
- `binding.language` (`omitempty`): the selection, sealed beside the plan hash. A default binding
  keeps its exact bytes and id (a test reproduces the pre-field encoding). `"all"` is not a legal
  value, so a selection has one spelling and one id. `recfmt.CheckLanguage`.
- `record-merge --language <name|all>` (default `all`, validated like a binding's): the merge job's
  own declaration of the selection, never read from a shard. Every shard binding's `language` must
  equal it or the run is void (B3), so blank bindings under `--language go` and `go` bindings under
  the default are both refused. With the default the plan must cover every graph package, so a
  language-restricted plan can never be read as `all`. With a selection, every plan package must be
  of that language and the plan must be closed on the merge's own graph (no prerequisite or dev
  dependency outside the plan), else void. Steps entries for packages the plan does not run are
  skipped under a selection; with `all` they are still refused (`expectedFor`, tested directly; the
  old `TestMergeRefusesAStepsExportThePlanDoesNotRun` failed at B1 first and was deleted).
- New fail-closed rule M16: an absent or untracked observation inside an unselected package's
  directory (nearest enclosing graph package) costs that action its record, via the fixed reason
  `foreign-package-read`. It is not an order dependence and does not void the run. Reading a foreign
  package's tracked sources is unaffected. The merge report prints a count of dropped actions.
- Tests: default binding bytes and ids, `PlanArgs` for both selections, disagreeing bindings, plan
  outside the selection, foreign absent/untracked/tracked reads, nearest-enclosing-package
  nesting, a shard claiming an unselected package (mutant killer), and a selection dimension in
  `FuzzMerge`'s oracle.
- `kiln:recorder` is unchanged: no action's verdict changes.
