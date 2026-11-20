## [0.1.0] - UNRELEASED

### Changed (classifier: ancestor nodes and absence under the fresh tmpfs, KILN04 D39)

- `internal/traced/host.go` `observeHost` takes `listing`. A present,
  non-listing look at a proper ancestor of an anchor (the checkout, a declared
  host root, `Home`, `Tmpdir`, the literal prefix of an `ephemeral` or
  `absent-ok` entry; `/` included) is no longer `host-read`. A listing, a child
  of an ancestor and a present sibling still are.
- An absent probe strictly inside the fresh `Home` or `Tmpdir` is no longer
  `host-read`; `underTmp` still excludes the checkout, declared roots and their
  ancestors.
- `internal/traced/traced.go`: `prepare` builds the `ancestors` set;
  `literalPrefix` finds an entry's anchor.
- `RecorderVersion` 6 to 7: the verdict for these actions changes, so every
  weak key changes.
- Spec: KILN04 D39 replaces the "other ancestors stay host-read" clause of
  section 5.5(c).

### Added

- `internal/traced/ancestor_test.go`; the differential fuzzer's oracle
  (`model_test.go`) models both rules.
