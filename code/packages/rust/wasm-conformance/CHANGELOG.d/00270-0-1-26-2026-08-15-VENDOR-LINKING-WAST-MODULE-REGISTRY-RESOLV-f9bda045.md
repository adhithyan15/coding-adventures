## 0.1.26 — 2026-08-15 — vendor linking.wast; module registry resolves $id (task #93)

### Fixed

- The module registry (`Executor`) now registers each module under its
  own script-level `$id` too (in addition to the existing `None`/"current
  module" slot), using `wasm-wast-parser`'s newly-captured `Directive::
  Module.id`. `Action::Invoke`/`Action::Get`'s `module: Option<String>`
  field and `Directive::Register`'s `module_name: Option<String>` field
  already carried a `$id` reference -- they just had nothing to resolve
  it against. This was the SOLE root cause of every one of the real,
  vendored `linking.wast` corpus file's 65 `assert_return` failures
  (confirmed via direct diagnostic: all 65 failed with the identical "no
  module registered as Some($id)" message before this fix); 48 of them
  now pass for real. The remaining 17 trace to a real, separate,
  already-documented limitation (`RegistryHost::resolve_memory`/
  `resolve_table`'s clone-not-share semantics for cross-instance
  memory/table imports) -- out of scope here, a distinct future epic.
- `Directive::Register`'s handling of an explicit `$id` target (as
  opposed to "register the current module") is now real instead of
  silently ignored.

### Added

- Vendored `linking.wast` (task #93) -- real cross-module linking was
  originally excluded from this crate's corpus (W05's own scope note:
  "needs heavier module-linking semantics"), but WASM05's real
  `HostInterface` link-failure path already provides exactly that. Of its
  71 modules, 2 import from `spectest` (this crate has no `spectest`
  host, by design) and grade `NotYetSupported`/collateral-fail; the rest
  exercise real, already-supported machinery. Lands at `assert_return`
  48/65, `assert_trap` 18/18, `assert_unlinkable` 49/50, `module` 16/17,
  `register` 6/8 -- see `tests/fixtures/testsuite/NOTICE` for the full
  vendoring rationale.
- Baseline regenerated: zero regressions in any of the other 61
  pre-existing vendored files (full before/after per-file/per-kind diff).

