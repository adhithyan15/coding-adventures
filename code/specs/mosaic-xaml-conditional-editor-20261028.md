# XAML conditional editor realization investigation (2026-10-28)

Status: proposed experiment; no acceptance claim. Tracks #16865 and epic #14267.

## Evidence and problem

The retained #18386 VisiCalc Debug candidate has a TextBox in every cell's collapsed If branch. The shared emitter lowers If/Else to ContentControl.Visibility, leaving inactive content and compiled Owner.EditContent bindings alive. Four warm whole-value updates measured EditContent setters at 10.5455-12.3265ms; commits at 9.0098-16.1100ms. The largest measured commit callback was41.3001ms, including18.7631ms apply-props and4.9668ms persistence. This small diagnostic identifies a candidate cost, not a percentile regression or proof of callback fan-out.

Native protocol/raw hashes: https://github.com/adhithyan15/coding-adventures/issues/16865#issuecomment-6464593245. Earlier matched scrolling: https://github.com/adhithyan15/coding-adventures/issues/16865#issuecomment-6464398996.

## Bounded experiment

1. Inspect current-main conditional lowering and generated compiled bindings. Establish whether inactive input branches receive editor text updates, without introducing application names or slot-name special cases.
2. Prototype conditional realization in mosaic-emit-xaml, using named x:Load containers controlled by the existing boolean expression. Preserve the outer single-content wrapper so deferred content is never a DataTemplate/UserControl root. Handle inverse branches and generated-name uniqueness. First prove whether unloading actually removes expensive target updates; x:Bind source tracking may remain.
3. Preserve the active branch while its condition stays true; do not rebuild on ordinary text updates. On false-to-true, bind current owner values and restore existing Loaded focus behavior. On true-to-false, teardown must not dispatch a stale edit. Determine lifecycle compatibility before choosing default scope; do not globally change all If branches without checking retained-state expectations.
4. If native compilation/interaction or binding costs falsify this approach, record the result and select a narrower shared editor-binding solution. Never patch generated application output.

Microsoft documents x:Load as conditional creation/removal with a named-element requirement, disallows template roots, and recommends measurement for list items: https://learn.microsoft.com/en-us/windows/apps/develop/platform/xaml/x-load-attribute. Placeholders and realization can increase costs, so success requires measured benefit.

## Validation and publication gates

- Regression fixtures: slot predicates, computed nested-row predicates, literal true/false, then/else, multi-child branches, nested conditions, multiple branches/names, layout slot placement. Include generated C# and actual WinUI markup compilation.
- Native generated VisiCalc: inactive cells do not acquire/edit text; formula staging/commit/recalculation; cell editor entry, caret, cancellation, switching cells, scrolling/recycled rows, restart persistence and keyboard edge navigation. Observe UI and accessibility. Preserve one Rust adapter/shared Mosaic UI.
- Compare same workload and managed configuration against retained baseline, separating warmups and detailed attribution from percentile samples. Record allocation, projection/realization and exact binaries. No acceptance closure from a small sample or callback timing alone.
- Full emitter tests, downstream application fixture checks, security review before push, README/changelog, exact-head CI. Keep one active VisiCalc PR. Downloaded release/design/persistence/accessibility gates remain open.

## Plan state

- [x] Epic and active PR audit; no active VisiCalc implementation PR.
- [x] Fresh origin/main isolated checkout; dirty original preserved.
- [x] Source confirms collapsed ContentControls retain editor bindings.
- [ ] Native candidate and meaningful regression tests.
- [ ] Performance and interaction validation; choose/reject realization strategy.
- [ ] Review, publish, CI and verified merge.