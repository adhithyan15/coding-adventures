# UI58 — `HostInput` gains a `disabled` prop

Issue: [#14772](https://github.com/adhithyan15/coding-adventures/issues/14772)

Extends [UI29](UI29-primitive-kernel.md)'s primitive kernel. `HostButton`,
`HostCheckbox` and `HostRadio` all lower a `disabled` prop natively;
`HostInput` has none, and text inputs are the only interactive primitive that
cannot be disabled.

## 1. The question this settles

Three toolkit components — `Input`, `Field`, `InputGroup` — declare a
`disabled` slot and forward it to `read-only`:

```
HostInput [ input ] (
  value : slot: value ,
  read-only : slot: disabled ,
  ...
)
```

`Input.mil` documents the slot as *"When true the input renders disabled and
stops firing onChange."* It renders read-only.

## 2. Why it is a workaround, not a slip

Measured across every backend's `emit_host_input`: **zero of eight accept a
`disabled` prop.**

| Backend | `disabled` | `read-only` |
| --- | :-: | :-: |
| html | — | yes |
| react | — | yes |
| webcomponent | — | yes |
| swiftui | — | yes |
| flutter | — | yes |
| xaml | — | yes |
| qt | — | — |
| compose | — | — |

`read-only` was the only thing available, so the toolkit used it.

## 3. They are different affordances

| | read-only | disabled |
| --- | --- | --- |
| keyboard focus | receives it | skipped |
| announced as | "editable text, read only" | "dimmed" / "unavailable" |
| text selection | allowed | not |
| submitted with a form | yes | no |

A person navigating by keyboard tabs into a field the application considers
unavailable, and a screen reader describes it as editable. Since #14774 the
toolkit also *dims* those inputs, which makes the mismatch worse rather than
better: a control that looks unavailable and still takes focus is more
confusing than one that looks available and takes focus.

### 3.1 The same layout already means different things per backend

SwiftUI lowers `read-only` to `.disabled(...)`, because SwiftUI's `TextField`
has no read-only state and `.disabled` is the closest available approximation.

So today the toolkit's `read-only : slot: disabled` produces a genuinely
disabled control on SwiftUI and a read-only one on html, react and XAML. One
authored line, two different affordances, decided by which backend is
compiling. That is the strongest argument that the missing prop — not the
toolkit's workaround — is the defect.

## 4. Decision

**`HostInput` accepts `disabled`, lowering to each platform's native disabled
state.** `read-only` stays, because a genuinely read-only field is a real and
different need.

### 4.1 Lowering

| Backend | `disabled` | `read-only` |
| --- | --- | --- |
| html | `disabled` attribute; `data-disabled` for a slot ref | `readonly` |
| react | `disabled={slot}` | `readOnly={slot}` |
| webcomponent | `disabled` attribute | `readonly` |
| swiftui | `.disabled(slot)` | `.disabled(slot)` — see §4.3 |
| flutter | `enabled: false` on the field | `readOnly: true` |
| xaml | `IsEnabled="{x:Bind Not(slot)}"` | `IsReadOnly` |
| qt | `enabled: !slot` | `readOnly: slot` |
| compose | `enabled = false` | `readOnly = true` |

Every one of these exists on its platform. This is a missing mapping, not a
platform limit — the same shape as #14717 and #14708.

### 4.2 Precedence when both are set

`disabled` wins. A disabled control is already non-editable, so `read-only`
adds nothing, and emitting both lets a backend that supports only one still do
the more restrictive thing.

### 4.3 SwiftUI's approximation is recorded, not hidden

SwiftUI has no read-only text field. `read-only` therefore continues to lower
to `.disabled(...)`, which over-restricts: the field stops taking focus when
the author asked only that it not be edited.

That is a real gap and it is **not** fixed here — it needs a custom field or a
focus-rejecting wrapper, which is its own work. What changes is that it stops
being invisible: with a real `disabled` prop, a component that wants disabled
says so, and only the genuinely-read-only case is left approximating.

### 4.4 Qt and Compose have neither today

Their `emit_host_input` reads no such prop at all. Both get `disabled` under
this spec; `read-only` for them is tracked separately, since adding it is not
required to make `disabled` correct.

## 5. Migration

1. Land the prop on all eight backends.
2. Point `Input`, `Field` and `InputGroup` at `disabled : slot: disabled`.
3. Leave `read-only` available for components that genuinely want it. None of
   the three do today — they were using it as a substitute.

Step 2 is deliberately separate: the prop is useless until every backend
accepts it, and switching the toolkit before then would make a disabled input
*less* restricted on whichever backend lagged.

## 6. Scope

**In:** the `disabled` prop on `HostInput`, its lowering on eight backends,
precedence against `read-only`, and the toolkit migration.

**Out:** giving SwiftUI a true read-only text field (§4.3). Adding `read-only`
to Qt and Compose (§4.4). `HostSlider` and `HostNumberInput`, which have the
same question and should be answered the same way once this lands — they are
left out only to keep the change reviewable, not because they differ.

## 7. Decisions completed while specifying

1. **Should `read-only` simply be renamed?** No. A read-only field is a real
   affordance — a form showing a value the person may copy but not edit — and
   several products will want it. The defect is that it was the *only* option,
   not that it exists.

2. **Should the toolkit switch in the same change?** No, and §5 says why: a
   partially-landed prop makes the control less restricted on the lagging
   backend, which is worse than the current consistent-but-wrong behaviour.

3. **Is dimming without disabling worse than neither?** Yes, and that is why
   this follows #14774 closely rather than waiting. A control that looks
   unavailable and still accepts focus and typing gives contradictory signals;
   before #14774 the input at least looked as interactive as it behaved.
