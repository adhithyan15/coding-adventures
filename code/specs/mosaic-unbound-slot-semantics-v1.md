# Mosaic unbound slot semantics v1: what an unbound slot on a package component means

**Status:** **PROPOSED, awaiting maintainer decision** (2026-10-26). §4 is a
recommendation, and §7 lists the questions to answer before the first
implementation PR. No code changes with this document.
**Tracked by:** E6 of the editor convergence audit
(`mosaic-editor-convergence-audit-v1.md`, #18370, risk R1), and
`MOSAIC-BACKLOG.md` EM-15.
**Builds on:**
- [UI13: mosmodel](UI13-mosmodel.md) §1, which already says "Slots without
  defaults are **required**: the compiler rejects a component instantiation
  that omits a required slot."
- [UI14: moslayout](UI14-moslayout.md) §2 (`Grid`) and §7.
- [UI34: `pkg::` reference syntax](UI34-moslayout-pkg-reference-syntax.md)
  §5.1 step 4 (call-site binding) and §7, which specifies a
  `SlotBindingMissing` error that was never built.
- The proposed `mosaic-package-publishing-v1.md` (#18358) §7.3, the
  breaking-change table.

---

## 0. Summary

The audit asked whether an unbound package slot takes its zero value (as
UI14 §2 says for `Grid`) or whether every slot addition is breaking. The
specs already answer this, and the answer is neither. **UI13 §1 says a slot
with a default is optional and a slot without one is required.** The
resolver already applies defaults: #11007 added that for `text`, `number`
and `bool`. The publishing proposal's table already assumes the same rule:
"Add a slot **with** a default: No".

What is broken is how the *required* half is enforced. Nothing checks it. An
unbound required slot is left in the inlined tree as a reference to a slot of
the *consumer*. Three things can then happen, depending on luck:

1. The consumer has no slot by that name. The build fails with a misleading
   `UnknownSlot` that names the consumer, not the package (probe D, P1).
2. The consumer has a slot by that name. The build **succeeds**, and the
   consumer's unrelated value flows into the package (P3).
3. The slot is used only inside an expression. The build **succeeds** on all
   9 backends, and the generated Swift and TypeScript refer to an undeclared
   identifier (P12).

Probe D also passes 9/9 once `body-placeholder` is declared `= ""` (probe
D-default). So DraftEditor 0.2.0 (E1) can be a compatible release, and
Journal does not have to change.

**Recommendation (PROPOSED): option (c), explicit defaults, which is UI13's
rule as written.** Build the check UI34 specified, so the rule is enforced
rather than accidental. Allow defaults on `list<T>` (`= []`) and `one-of`,
so that every common kind of slot can be added compatibly. Add an
accessible-name rule so that a default can never leave a control unnamed.

---

## 1. Current behaviour, with evidence

### 1.1 How slots are declared

A slot is declared in the component's `.mil`:

```
slot_decl    = KEYWORD NAME COLON slot_type [ EQUALS slot_default ] SEMICOLON ;   # mosmodel.grammar:54
slot_default = STRING | NUMBER | KEYWORD ;                                         # mosmodel.grammar:94
```

- `mosmodel-compiler/src/lib.rs:490` sets `required = default.is_none()`.
  `SlotDecl` carries both `required` and `default: Option<SlotDefault>`, and
  both go into the interface descriptor JSON.
- `SlotDefault` has three variants only: `Text`, `Number` and `Bool`
  (`lib.rs:215`).
- `validate_slot_default` (`lib.rs:1013`) rejects a default on `image`,
  `color`, `list<T>` and component types with `NoDefaultForType`. It rejects a
  type mismatch with `InvalidDefault`. This matches UI13 §5, validation rule 3, and §8.
- **`one-of` cannot have a default.** A bare value (`= calm`) is a `NAME`,
  which `slot_default` does not admit, so it is a parse error. A string
  (`= "calm"`) parses but fails `InvalidDefault` (P6, P6b).
- **`list<T>` cannot have a default.** `[` does not lex in `.mil` (P5a).

So slots **can already declare defaults**, but only for the three scalar
types.

### 1.2 What happens at a call site

A `pkg::P::C ( … )` reference is inlined by
`LayoutPackageResolver::substitute`
(`mosaic-package-resolver/src/lib.rs:721`):

1. `build_default_binding_map` (`:970`) maps every defaulted slot to its
   default literal.
2. The call-site props are layered on top (`:741`). A call-site value wins.
3. `rewrite_bindings` (`:992`) replaces each `slot: x` in the inlined tree
   with its binding. **A `slot: x` with no binding is left as it is.**
   Expressions are rewritten identifier by identifier (`:1110`), and an
   unbound identifier is also left as it is.
4. Emits are treated differently. An unforwarded emit is *removed*
   (`node.props.retain`, `:996`), so emits are optional by construction. An
   unfilled `slot: children` mount is removed too (UI29-2; P11), so `node` and
   `list<node>` child slots are optional by construction as well.
5. Nothing compares the call-site props against the callee's interface. A
   binding whose name matches no slot is silently ignored (P4), and a bound
   value is not type-checked (P13).

Afterwards, the artifact builder's `resolve_layout_package_references`
(`mosaic-package-artifact-builder/src/lib.rs:10061`) re-validates the
inlined tree against the **consumer's** descriptor (`:10083`). The leftover
`slot: x` is then reported by `moslayout_compiler::validate`
(`moslayout-compiler/src/lib.rs:938`) as:

```
pipeline error for component 'JournalApp': CompileError { kind: UnknownSlot,
  message: "Unknown slot 'body-placeholder' referenced in layout — not declared in .mil" }
```

The message is wrong on three counts. It names the consumer, not
`mosaic-pkg-draft-editor::DraftEditor`. The slot *is* declared, in the
package's `.mil`. And the builder reports only `errs[0]`. This is the only
place a missing binding is caught, and it catches it only when the consumer
happens not to have a slot of the same name.

### 1.3 Probes

Built with `mosaic-compile pkg <consumer> --backend <b>` (debug build at
`origin/main` d096a82430b), on all 9 backends where marked "9". Otherwise
React and SwiftUI were built, and the result was the same on both. The shared
resolver runs before any emitter, so the result does not depend on the
backend except where generated code is inspected. Nothing was committed. The
harness is in Appendix A.

| Probe | Package change (consumer unchanged unless stated) | Builds | What happens |
| --- | --- | :-: | --- |
| P0 | Baseline: `Card { slot title : text ; }` bound by the consumer | 9/9 | |
| **P1** | Add `slot hint : text ;`, used by a `Text` | **0/9** | `UnknownSlot: Unknown slot 'hint' … not declared in .mil` |
| **D-required** | Real `DraftEditor` with `slot body-placeholder : text ;` bound to the body's `placeholder`, compiled through real `journal-app` | **0/9** | Reproduces the audit's probe D exactly |
| **D-default** | The same, declared `slot body-placeholder : text = "" ;` | **9/9** | Journal compiles unchanged |
| P2 | Add `slot hint : text = "" ;` | 9/9 | React emits `<span>{""}</span>` |
| **P3** | P1's required `hint`, and the consumer happens to declare its own `slot hint : text` | **9/9** | **Silent capture.** The package's `card-hint` renders the consumer's `hint` (`<span>{hint}</span>`), which was never bound |
| P4 | The call site binds `subtitel: "typo"`, which is not a slot | 2/2 | Silently ignored |
| P5a | `slot tags : list<text> = [] ;` | 0/2 | `LexerError … Unexpected sequence '['` |
| P5b | Add `slot tags : list<text> ;`, used by a `For` | 0/2 | `UnknownSlot 'tags'`. A list slot can never be added compatibly |
| P6 / P6b | `slot tone : one-of calm loud = calm ;` / `= "calm"` | 0/2 | Parse error / `InvalidDefault` |
| **P7** | `slot field-label : text = "" ;` bound to `HostInput ( a11y-label: … )` | 2/2, and **5/5 native-complete with 0 degradations** | React `aria-label=""`, SwiftUI `.accessibilityLabel(Text(verbatim: ""))`. **An unnamed field passes every gate.** |
| P8 | Add `emit onPick ;`, not forwarded | 2/2 | The emit is dropped. Adding an event is compatible |
| P9 / P9b | `slot compact : bool ;` gating an `If`, unbound / `= false` | 0/2 / 2/2 | `UnknownSlot` / the branch folds away |
| P10 | `slot tint : color = "#ff0000" ;` | 0/2 | `NoDefaultForType` |
| P11 | Add `slot children : list<node> ;` with a `slot: children` mount, unfilled | 2/2 | The mount is removed |
| **P12** | `slot count : number ;` used only in `If ( when: ( count > 0 ) )` | **9/9** | **Leaks into generated code.** React `{( count > 0 ) && …}` gives `tsc` `TS2304: Cannot find name 'count'`. SwiftUI `let _mosaicCondition0: Bool = count > 0` gives `swiftc` `cannot find 'count' in scope` |
| P13 | Bind a `list<text>` slot to a `text` slot, and `"yes"` to a `bool` | 2/2 | Not type-checked |
| **P14** | `slot tone : one-of calm loud ;` used only in `If ( when: ( tone == "calm" ) )`, and the call site passes the literal `tone: calm` | **2/2** | **The same leak as P12, from a literal.** The bare `Keyword` is spliced into the expression as an identifier (`layout_prop_value_as_expression`, resolver `lib.rs:1176`). React emits `{( calm == "calm" ) && …}` and `tsc` reports `TS2304: Cannot find name 'calm'`. SwiftUI emits `calm == "calm"` and `swiftc` reports `cannot find 'calm' in scope`. This happens today, without any default |

### 1.4 What the specs say

There is no conflict between specs. What conflicts is the specs and the
implementation.

- **UI13 §1** is the rule for components: required unless defaulted, and
  "the compiler rejects a component instantiation that omits a required
  slot". This is option (c).
- **UI14 §2**'s "All bindings are optional; unbound slots use zero values" is
  about the props of the `Grid` *primitive* (`headers:`, `rows:` …). It says
  what a primitive does when a layout leaves one of its props unset. It is not
  about the slots of a component, and the audit's R1 read it more widely than
  it was written. It should say so (§5.2).
- **UI34 §7** specifies `SlotBindingMissing { package, component, slot }`:
  "Call site did not supply a required slot." `LayoutResolveError` has no
  such variant (§1.2). UI34's suggested fix, "mark it optional in the
  component's `.mil`", refers to a marker that does not exist. The marker is
  `= <default>`.
- **Publishing §7.3** already has "Add a slot **with** a default: No" and
  "Add a slot with no default, or remove a default: **Yes**". Both rows are
  consistent with (c). The table is incomplete, though (§5.4), and two of its
  "Yes" rows are not true today. "Remove or rename a slot: call-site bindings
  fail" is false: the binding is ignored (P4). "Change a slot's type: bound
  values no longer type-check" is also false: nothing type-checks them (P13).

### 1.5 The other boundary: an exported component's host props

A component compiled as an export (an app root, or a component previewed on
its own) gets its slot values from the host at run time, not from a call
site. That boundary already distinguishes required slots from defaulted ones
on every backend:

| Backend | Defaulted slot | Required slot the host omits |
| --- | --- | --- |
| React, Electron | default literal (`mosaic-emit-react/src/pipeline.rs:540`). The prop is `?` when it is not required (`:815`, `lib.rs:327`) | Not optional in the TS props type |
| Web Components | default literal (`mosaic-emit-webcomponent/src/pipeline.rs:696`) | Backing field initial value |
| HTML | default in the JSON state (`mosaic-emit-html/src/pipeline.rs:515`) | Absent from the state |
| SwiftUI | `optionalString(…) ?? <default>` (`mosaic-emit-swiftui/src/pipeline.rs:809`) | `requiredString` → `preconditionFailure("Mosaic runtime omitted required prop …")` (`:1181`) |
| Compose | non-null parameter with a default (`mosaic-emit-compose/src/pipeline.rs:3315`, `:9343`) | Non-null parameter, no default |
| Flutter | default value (`mosaic-emit-flutter/src/pipeline.rs:1438`, `:1646`) | Required parameter |
| Qt | left out of `requiredProps` (`mosaic-emit-qt/src/pipeline.rs:1140`) | `configureRequiredProps`, and the host refuses to render |
| XAML | left out of `RequiredProps` (`mosaic-emit-xaml/src/pipeline.rs:10120`) | `ApplyRequiredProps(…, strict: true)`, and the host refuses to render |

So every backend can already represent "required" and "defaulted" for the
three scalar types. Anything that adds a default type, such as `[]` or a
`one-of` value, must add a case to each of these eight functions. Electron
uses React's.

### 1.6 Inventory

- `code/packages/mosaic`: 24 packages, 61 `.mil` files, **575 slots, 22 of
  them defaulted**. Fifteen of those are in `mosaic-std-controls`, and seven
  are in `mosaic-pkg-grid`. By type: text 375, **list 73**, bool 64,
  number 55, **one-of 8**. No `image`, `color` or component-typed slots.
- All of `code/`: 119 `.mil` files, 1,181 slots, 44 defaulted, 135 lists.
- **Defaults that already feed an accessible name:** only `mosaic-pkg-grid`'s
  `editor-label : text = "Cell value"`, in `Grid`, `Cell` and
  `RowHeaderGrid`. It is non-empty, so it is safe, but it is an untranslated
  English literal.
- **Call sites that pass `""` to a label-gated slot:** 7. All are toolkit
  `EmptyState ( action-label : "" )`, in `JournalApp.mll` and `TaskApp.mll`.
  `EmptyState.mll:22` wraps the button in `If ( when: slot: action-label )`,
  so the `""` removes the button, and `fold_constant_conditionals` does that
  at compile time.

Today 14.1% of package slots (81 of 575, `list` and `one-of`) cannot be added
compatibly under *any* reading, because they cannot have a default.

---

## 2. The options

- **(a) Required by default, with no way out.** Today's enforcement, made
  explicit: every unbound slot is an error, and adding any slot is breaking.
  Existing defaults would have to go, or be treated as documentation only.
- **(b) Zero values.** An unbound slot takes its type's zero value: text
  `""`, number `0`, bool `false`, list `[]`, `one-of` its first value, and
  `node` nothing. Every slot becomes optional.
- **(c) Explicit defaults.** `slot x : T = <default> ;` is optional and
  unbound means the default. A slot without a default is required, and
  omitting it is a compile error. This is UI13 §1 as written.
- **(d) An `optional` marker.** `slot x : T optional ;` (or `T?`) means
  "unbound means the zero value", opted into per slot. Unmarked slots are
  required. This is (b) per slot, and alongside existing defaults it is a
  second spelling of (c).

---

## 3. Assessment

### 3.1 At a glance

| | (a) required | (b) zero values | (c) explicit defaults | (d) `optional` |
| --- | --- | --- | --- | --- |
| Adding a slot | always **breaking** | always compatible | compatible **iff** defaulted | compatible iff marked |
| An unbound name slot | error (safe) | **silently `""`** | error unless defaulted. An empty default is refused (R4) | **silently `""`** unless the same rule is added |
| A forgotten binding | error | **silent** | error | error unless marked |
| Grammar change | none | none | `[]` and `one-of` values in `slot_default` | a new keyword |
| Inline-time change | `SlotBindingMissing` | zero-literal substitution, plus a `[]` form | `SlotBindingMissing`, plus `[]` and `one-of` defaults | both |
| Emitters (9) | none | all 8 standalone default paths make every prop optional. Qt, XAML and SwiftUI lose their required-prop refusal | add `[]` and `one-of` to the 8 default paths | as (b), for marked slots |
| Existing packages | 22 defaults lose meaning | 553 required slots become optional silently | **none** | none |
| Reversible later? | yes | **no**: going back is breaking for everyone who relied on omission | yes | yes |

### 3.2 (a) Required by default

- **Publishing §7.3:** every slot addition is a breaking (major, or pre-1.0
  minor) bump. "Add a slot with a default: No" would be deleted. That
  contradicts the table and UI13.
- **Accessibility:** safe at compile time, since every label must be bound.
  It does not stop a consumer binding `""` (the audit's Engram case,
  `unwrap_or_default`).
- **Emitters:** none.
- **Existing packages:** `mosaic-std-controls` and `mosaic-pkg-grid` rely on
  their 22 defaults. Every one of their call sites that omits one would break.
- **Migration:** a mass binding PR across consumers, then a bump of both
  packages.
- **Tooling:** `SlotBindingMissing` is still needed, because today's
  `UnknownSlot` is misleading and P3/P12 are silent.
- **Verdict:** throws away a working feature to make the cost of change
  higher. Rejected.

### 3.3 (b) Zero values

- **Publishing §7.3:** every slot addition is compatible, and so is *removing
  a default*. But "a consumer forgot to bind X" becomes undetectable, and so
  does "the package renamed X". With P4 unfixed, a rename turns into a silent
  value reset. The interface-diff check (publishing Phase 7) loses its main
  signal.
- **Accessibility:** **the worst option.** Under (b), DraftEditor's
  `body-label` unbound is a multi-line field with no accessible name on all
  9 backends, and P7 shows that the native-complete gate reports nothing.
  The kernel has made an unnamed control a compile error before, for
  `HostNavigationSplit` without a `pane-title` (`moslayout-compiler/src/lib.rs`,
  UI29-6 §4.1). (b) would reintroduce the hazard everywhere.
- **Emitters:** every standalone default path (§1.5) has to treat every
  prop as optional with a zero fallback. Qt's `requiredProps`, XAML's strict
  `ApplyRequiredProps` and SwiftUI's `requiredString` all lose their reason
  to exist, so a Rust host that forgets a prop renders blank instead of
  failing. The inliner needs a literal empty-list form, because
  `LayoutPropValue` has none, and every backend must lower it wherever a
  list is consumed: `For`, `HostSelect` options, `Grid` rows, `Path` points.
  A zero value for `one-of`, "the first value", is arbitrary. For `color` and
  `image` there is no honest zero.
- **Existing packages:** no build changes. 553 slots change meaning without
  any file changing.
- **Migration:** none, and it cannot be undone. Moving from (b) to (c) later
  breaks every consumer that relied on an omission.
- **Tooling:** would need an "unbound slot" warning to recover what (c) gives
  as an error. That is (c) with the polarity reversed and a weaker default.
- **Verdict:** rejected.

### 3.4 (c) Explicit defaults

- **Publishing §7.3:** "Add a slot with a default: compatible" and "Add a
  slot without one: breaking" become *true*. Once R3 (§4) lands, they hold
  for lists and `one-of` too. The rows needed are listed in §5.4.
- **Accessibility:** safe with rule R4. A package author cannot default a
  name slot to an empty or whitespace-only string. The one exception is an
  exact `""` with the named element gated on that same slot, as DraftEditor
  does. And an empty literal (after trimming whitespace) left on a name prop
  after inlining is an error.
- **Emitters:** for text, number and bool, nothing: the resolver and all 8
  standalone default paths already work (§1.2, §1.5; P2, P9b, D-default).
  `one-of` defaults need a default literal in the 8 paths. At inline time a
  default is substituted the way a caller's literal is. That already works in
  prop position, but **not inside an expression**: a literal `one-of` value is
  spliced in as a bare identifier (P14). R3 fixes that for defaults and
  call-site literals together (F5).
  `[]` needs the 8 paths, plus inline-time folding of `For ( each: [] )`, plus
  a literal form for any other list use (F6).
- **Existing packages:** none. The 22 defaults keep their meaning, and the 553
  required slots stay required. Every current build already binds them,
  except any latent captures like P3, which F1 will surface as errors.
- **Migration:** none required. A package can opt in by adding a default,
  which is a compatible minor bump: `EmptyState.action-label = ""`, and
  DraftEditor's new `body-placeholder = ""`. Consumers can then drop
  redundant `""` bindings.
- **Tooling:** `SlotBindingMissing` (F1), `UnknownSlotBinding` (F2), the
  accessible-name rules (F4), and the interface-diff rows (publishing
  Phase 7).

### 3.5 (d) An `optional` marker

- **Publishing §7.3:** like (c), with a second row for "add an optional
  slot".
- **Accessibility:** `slot body-label : text optional` is exactly (b)'s
  hazard. R4 would be needed again.
- **Emitters:** as (b), for marked slots.
- **Existing packages:** none.
- **Migration:** none. Every package then has two ways to say "optional",
  `= ""` and `optional`, which differ only in whether the author wrote the
  value.
- **Tooling:** the same as (c), plus deciding whether `optional = x` is
  legal.
- **Verdict:** buys nothing for scalars that `= ""`, `= 0` and `= false` do
  not already do. Its one real use is a type with no literal default. For
  lists, `= []` (R3) covers that, and it is the same form every other
  default uses. Rejected in favour of (c), with `optional` kept as an open
  question for `color` and `image` (Q8).

---

## 4. Recommendation: PROPOSED

**Adopt (c), which is UI13 §1's existing rule, and make the toolchain
enforce it.** Concretely:

- **R1. A required slot must be bound.** At each `pkg::P::C` call site the
  resolver checks the callee's *interface*, not its usage. Every slot with no
  default must be bound, or the result is
  `SlotBindingMissing { consumer, package, component, slot }`, before any
  substitution. Checking the interface rather than the layout is what closes
  P3 (capture) and P12 (expression leak). Both happen because an unbound name
  survives into the consumer's scope. The message names the package and
  suggests the fix: "bind `body-placeholder` at the call site, or give it a
  default in `DraftEditor.mil`". All errors are reported, not `errs[0]`.
- **R2. A binding must name a slot or an emit.** A call-site prop that names
  neither a callee slot nor a callee emit is `UnknownSlotBinding`, with a
  closest-name hint (P4). Without it, "removing or renaming a slot is
  breaking" is not true, and a typo is a silent drop.
- **R3. Defaults for every type that has an honest one.**
  - `list<T> = []`: only the empty list. The inliner folds
    `For ( each: [] ) { … }` away, as `fold_constant_conditionals` folds
    `If ( when: false )`.
  - `one-of a b c = b`: a bare value, which must be one of the declared
    values. **Substitution rule.** In prop position the default becomes
    `LayoutPropValue::Keyword("b")`, exactly what a caller's `tone: b`
    produces today. Inside an expression, the value of a `one-of` slot,
    whether it comes from a default or a call-site literal, is always spliced
    as a quoted string literal (`"b"`) and never as a bare identifier.
    `rewrite_expression_bindings` gets this from the callee slot's type, so
    only `true` and `false` for `bool` slots and numbers stay unquoted.
    Without the rule, a defaulted `one-of` reproduces P14's undeclared
    identifier in every consumer that omits it.
  - `node` and `list<node>` child slots are already optional (P11), and the
    spec says so.
  - `image`, `color` and component types stay non-defaultable (Q8). None
    exists in any package today.
- **R4. A default cannot leave a control unnamed.** Let a *name prop* be a
  kernel prop that becomes an accessible name. At least these: `HostInput`,
  `HostSlider`, `HostButton`, `Text` and `HostTable` `a11y-label`;
  `HostButton`, `HostLink`, `HostCheckbox`, `HostRadio`, `HostDraggable` and
  `Icon` `label`; `HostDialog` `title`; and `HostNavigationSplit`
  `pane-title`. F4 pins the list in one constant, checked against UI29 §2.1,
  and shares it with the native-degradation analyser.
  - **"Empty" means empty after trimming Unicode whitespace** (Rust
    `str::trim`, i.e. the `White_Space` property). `" "`, `"\n"` and
    `"\u{3000}"` name nothing, so a screen reader announces them as an
    unnamed control. They are also *truthy*, so an `If ( when: slot: x )` gate
    keeps the element rather than removing it.
  - (i) **Package time.** In the package's own layout, a slot whose default
    is empty and whose value reaches a name prop is
    `EmptyAccessibleNameDefault`. There is one exemption: a default of
    **exactly** `""`, where every use sits inside
    `If ( when: slot: <that slot> )`. That is the label-gated pattern of
    `DraftEditor` and `EmptyState`, where `""` is falsy and removes the named
    element. A whitespace-only default is never exempt, gated or not, because
    the gate lets it through.
  - (ii) **After inlining** and constant folding, a name prop holding a
    literal that is empty is `EmptyAccessibleName`. This catches a consumer
    binding `""` or `" "` to an ungated name, and `" "` to a gated one (the
    gate does not fold away). An exact `""` under its own gate has already
    folded away, so (ii) needs no exemption. The 7 existing
    `action-label : ""` sites pass.
  - (iii) A non-empty literal default on a name slot is the warning
    `LiteralAccessibleNameDefault`, because it is untranslated English. Grid's
    `"Cell value"` is the only instance today.
  - A host that passes `""` at run time cannot be caught statically. That
    stays a host rule, as the audit says for Engram (§4.2).
- **R5. The version rule.** Adding a slot with a default, or adding a default
  to an existing slot, is compatible. Adding a slot without a default, or
  removing a default, is breaking. §5.4 has the rows.

**For E1:** declare `slot body-placeholder : text = "" ;`.
`placeholder` is not a name prop, so R4 does not apply. DraftEditor 0.2.0
is then a compatible minor release, and Journal does not need to bind it
(D-default, 9/9). The audit's §4.1 "this PR must also bind it in Journal"
and §4.3's "the breaking slot addition is cheapest now" no longer apply.

---

## 5. Exact spec changes needed elsewhere

These are made in follow-up PRs once §4 is decided, not here.

### 5.1 UI13 (`UI13-mosmodel.md`)

1. §1 Syntax: after "the compiler rejects a component instantiation that
   omits a required slot", add: "That error is `SlotBindingMissing`
   (UI34 §7). A call site that binds a name the component does not declare
   is `UnknownSlotBinding`. `node` and `list<node>` slots used as a child
   mount (UI29-2) are optional without a default, and an unfilled mount
   renders nothing."
2. §5 Validation rule 3: replace the list of non-defaultable types with:
   "`list<T>` may default to `[]` only. `one-of` may default to one of its
   declared values, written bare. `image`, `color` and component types may
   not have inline defaults."
3. §4 Grammar: `slot_default = STRING | NUMBER | BOOL_LIT | NAME | LBRACKET RBRACKET ;`,
   with the lexer gaining `[` and `]`. Note the regenerate-every-projection
   lesson.
4. §6 error table: `InvalidDefault` covers a `one-of` value outside its set,
   and a non-empty list.
5. §8 Out of scope: remove `list` from "Default slot values for …".
6. The descriptor JSON example: show `"default": []` and a `one-of`
   default.

### 5.2 UI14 (`UI14-moslayout.md`)

1. §2 `Grid`: change "unbound slots use zero values" to "an unset `Grid`
   prop uses its zero value". Add: "This is about a primitive's props. A
   component's unbound slots follow UI13 §1."
2. §7 Validation: add "8. **Accessible names.** A slot whose default is
   empty after trimming Unicode whitespace may not reach a name prop (list in
   UI29 §2.1). The one exception is a default of exactly `""` where every such
   use is inside `If ( when: slot: <that slot> )`. After inlining, a name prop
   holding such an empty literal is an error, gated or not."
3. §9 Errors: add `EmptyAccessibleNameDefault` and the warning
   `LiteralAccessibleNameDefault`.

### 5.3 UI34 (`UI34-moslayout-pkg-reference-syntax.md`)

1. §5.1 step 4, add these bullets. "Every callee slot without a default must
   be bound, checked against the callee's interface, before substitution
   (`SlotBindingMissing`). A call-site prop must name a callee slot or emit
   (`UnknownSlotBinding`). An omitted defaulted slot takes its default. An
   omitted emit is dropped. An unfilled child mount is removed. After
   substitution and constant folding, a name prop holding a literal that is
   empty after trimming Unicode whitespace is `EmptyAccessibleName`."
2. §7: the `SlotBindingMissing` fix text becomes "Bind the slot at the call
   site, or give it a default in the component's `.mil`". Add
   `UnknownSlotBinding` and `EmptyAccessibleName` rows. Add the note: "Until
   built, a missing binding surfaced as the consumer's `UnknownSlot`, or not
   at all (unbound-slot-semantics-v1 §1)."

### 5.4 Publishing (`mosaic-package-publishing-v1.md`, #18358) §7.3

Do not edit the file here. Proposed table edits:

| Change | Breaking? | Why |
| --- | --- | --- |
| Add a slot **with** a default (any defaultable type, including `list = []` and `one-of`) | No | *(replaces the existing row)* Omitted at a call site, it takes the default (UI13 §1) |
| Add a slot with no default, or remove a default | **Yes** | *(existing row, reason sharpened)* Call sites that omit it fail with `SlotBindingMissing` |
| Add a default to an existing required slot | No | Call sites that bound it are unchanged |
| Change a slot's default value | No (minor), called out in the CHANGELOG | Every call site that omits it changes behaviour, but none fails to compile (Q2) |
| Add an `image`, `color` or component-typed slot | **Yes** | These types cannot have a default (UI13 §5) |
| Add a `node` / `list<node>` child-mount slot | No | An unfilled mount renders nothing (UI29-2) |
| Default a slot that feeds an accessible name to an empty or whitespace-only string | Not allowed, except an exact `""` gated on itself | `EmptyAccessibleNameDefault` (UI14 §7) |

Notes:

- The existing "Remove or rename a slot: **Yes**" row needs a footnote:
  "true once `UnknownSlotBinding` exists (F2). Until then a removed slot's
  binding is silently ignored."
- The "Change a slot's type" row needs the same footnote for F3.
- §7.3 *Enforcement*: the interface diff compares `required`/`default` per
  slot.

### 5.5 Component program (`mosaic-component-program-v1.md`) §7

Add two items to the definition of done:

- [ ] Every slot added after the component's first release has a default, or
      the release is a breaking bump.
- [ ] No slot that feeds an accessible name defaults to an empty or
      whitespace-only string, except an exact `""` gated on itself.

### 5.6 Editor convergence audit (#18370)

- R1: replace "UI14 §2 says … nothing found … says this differs" with a
  pointer to this spec: UI13 §1 governs, and a defaulted slot is compatible.
- §4.1: DraftEditor 0.2.0 declares `slot body-placeholder : text = "" ;`.
- §4.2 Journal: "Nothing changes". §4.3 step 1: compatible minor release.
- §6 E6: point at this spec.

---

## 6. Follow-up implementation PRs

One per PR, in this order. F1 and F2 are independent of the maintainer's
answers to Q2 to Q8, because they enforce what UI13 and UI34 already say.

| # | PR | Where | Size | Proves it with |
| --- | --- | --- | :-: | --- |
| **F1** | `SlotBindingMissing`: check every required callee slot is bound, against the interface, before substitution; report every error | `mosaic-package-resolver` (`substitute`), `LayoutResolveError`; builder stops reporting only `errs[0]` | S | P1, P3, P5b, P9 and P12 become `SlotBindingMissing` naming the package; P2, P9b and D-default still pass; every package and program in the repo still builds on all 9 backends (any new failure is a latent capture, fixed in the same PR) |
| **F2** | `UnknownSlotBinding`, with a closest-name hint | resolver | S | P4 fails; a full repo build finds existing typos, if any |
| F3 | Call-site type check (`SlotBindingTypeMismatch`): literal and slot-ref kinds against the callee slot type | resolver, using the consumer descriptor | M | P13 fails. Adjacent to E6, not required by it (Q7) |
| **F4** | Accessible-name rules R4 (i) to (iii): one `ACCESSIBLE_NAME_PROPS` constant; (i) in `moslayout_compiler::validate` with descriptor defaults; (ii) after inlining in `resolve_layout_package_references` | `moslayout-compiler`, artifact builder | S–M | P7 fails; so does P7 with a whitespace-only default (`= " "`, `= "\n"`), gated or ungated, and so does a call-site `label: " "`; the DraftEditor and EmptyState gated `""` patterns pass; Grid warns |
| F5 | `one-of` defaults: `slot_default` admits `NAME`, validated against the set; the default becomes `Keyword` in prop position and a quoted string literal inside an expression (R3), and the same quoting applies to call-site `one-of` literals (fixes P14); plus the 8 standalone default paths | grammar (regenerate projections), `mosmodel-compiler`, resolver (`rewrite_expression_bindings`), 8 emitters | S–M | P6 passes on 9/9; a value outside the set is `InvalidDefault`; P14, and P14 with `tone` omitted and `= calm` defaulted, build on 9/9, and the generated React passes `tsc --noEmit` and the SwiftUI passes `swiftc -typecheck` (P12-style) |
| F6 | `list<T> = []`: lexer `[ ]`, `SlotDefault::EmptyList`, inline folding of `For ( each: [] )`, a literal form for other list uses, and the 8 standalone default paths | grammar, `mosmodel-compiler`, resolver, 8 emitters | M | P5a passes on 9/9; a `For` over an unbound defaulted list renders nothing |
| F7 | Adopt it: DraftEditor `body-placeholder = ""` (this is E1); toolkit `EmptyState.action-label = ""`, then drop the 7 redundant `""` bindings | packages | S each | Native gates unchanged |
| F8 | Docs: §5.1 to §5.3 and §5.5 (and §5.4 into #18358 if it is still open) | specs | S | |

F1 can land before the maintainer answers §7. It changes no build that passes
today except latent capture bugs, and those are wrong already.

---

## 7. Open questions for the maintainer

1. **Confirm (c)** as the rule for package components. It is UI13 §1's
   existing text, so confirming it means "enforce what is written".
2. **Is changing a default breaking?** Proposed: no (minor), with a
   CHANGELOG call-out. A gating slot is the hard case: changing
   `action-label = ""` to `= "Retry"` makes a button appear at every call site
   that omits it.
3. **`EmptyAccessibleName` in every profile, or only native-complete?**
   Proposed: every profile, as a compile error, as `HostNavigationSplit`'s
   `pane-title` already is.
4. **A non-empty literal default on a name slot:** a warning (proposed),
   an error (forcing localisable labels to be required), or nothing?
5. **List defaults:** only `[]` (proposed), or literal lists such as
   `= ["a", "b"]`?
6. **Must a required slot that the package's layout never reads still be
   bound?** Proposed: yes. The interface is the contract, and checking the
   interface, not usage, is what closes the capture and expression-leak holes.
7. **Is F3 (call-site type checking) part of E6,** or a separate backlog
   item? The publishing table's "change a slot's type" row is not true without
   it.
8. **`image` and `color`:** stay non-defaultable (proposed, since none are
   used), or take an `optional` marker that means "absent"?
9. **The host boundary:** SwiftUI crashes (`preconditionFailure`) when a
   host omits a required prop, while Qt and XAML refuse with a status
   message. Should SwiftUI, Compose and Flutter refuse the same way? This is
   outside E6, but it is the same "required" rule at run time.

---

## Appendix A: probe harness

Scratch only. Build `mosaic-compile` with a private `CARGO_TARGET_DIR`, then
for each probe write two sibling packages and compile the consumer:

```sh
# pkgs/mosaic-pkg-lib/{mosaic-package.toml, src/Card.mil, src/Card.mll}   exports Card
# pkgs/probe-app/{mosaic-package.toml, src/ProbeApp.mil, src/ProbeApp.mll} depends on mosaic-pkg-lib
#   ProbeApp.mll: layout ProbeApp { Column [ app-root ] {
#                   pkg::mosaic-pkg-lib::Card ( title: slot: heading ) } }
for b in react electron swiftui qt webcomponent html xaml flutter compose; do
  mosaic-compile pkg pkgs/probe-app --backend "$b" --output "out/$b"
done
```

The sibling directory is found through `default_package_search_paths`
(artifact builder `lib.rs:10456`: the package root's parent). D-required and
D-default copy `journal-app`, `mosaic-pkg-draft-editor` and
`mosaic-pkg-toolkit` into one scratch directory. They add
`slot body-placeholder : text [= ""] ;` after `body-value` in
`DraftEditor.mil`, and `placeholder : slot: body-placeholder ,` to the body
`HostInput`. P12's generated sources were checked with
`tsc --noEmit --jsx preserve` and `swiftc -typecheck`.
