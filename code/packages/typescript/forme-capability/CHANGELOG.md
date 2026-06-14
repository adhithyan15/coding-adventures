# Changelog — @coding-adventures/forme-capability

## 0.1.0 — 2026-05-15

Initial release. Implements the FM01 §5 capability layer.

### Added

- `Capability` type alias (plain `string`).
- `ParsedCapability` interface — view of a parsed capability with
  realm / scope / detail / wildcard fields.
- `parseCapability(cap)` — strict parser; throws `RangeError` on
  malformed input (wrong segment count, empty segment, whitespace).
- `tryParseCapability(cap)` — returns `null` instead of throwing.
- `matchesCapability(declared, requested)` — enforcement-time
  predicate. Implements scope wildcard, detail wildcard, and the
  network-realm host-hierarchy semantics from FM01 §4.8.2.
  Returns `false` (rather than throwing) on malformed input.
- `KERNEL_REALMS` — frozen list of the 9 kernel-blessed realm names.
- `KernelRealm` type alias.
- `FIRST_PARTY_ONLY` — capabilities that must not be granted to
  third-party plugins (`system:shell`, `system:time-nondeterministic`).
- `SENSITIVE` — capabilities that always warrant a stark install-time
  warning (`network:*`, `env:*`, `filesystem:user`, `system:shell`).
- `isKernelRealm`, `isFirstPartyOnly`, `isSensitive` predicates.

### Spec adherence

No deliberate divergences from FM01 §5.

### Notes

- DNS comparisons in the network realm are case-insensitive
  (`Foo.Com` matches `API.FOO.COM`), matching DNS spec behaviour.
- Substring traps are explicitly tested: `network:foo.com` does NOT
  cover `network:xfoo.com`. The check uses `endsWith("." + host)`,
  not `endsWith(host)`.
- 2-segment and 3-segment capability forms are not interchangeable.
  A scheme-restricted request requires a scheme-aware declaration.
