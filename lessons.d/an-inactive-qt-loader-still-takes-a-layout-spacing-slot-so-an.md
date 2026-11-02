---
category: Mosaic compiler pipeline
---

# An inactive Qt Loader still takes a layout spacing slot, so an If/Else pair in a spaced layout carries a gap next to nothing

**What happened.** Mosaic lowers `If` / `Else` to two `Loader`s with
opposite `active` predicates (UI29 §3.2). Only one is ever live, so it is
tempting to assume the pair occupies exactly one slot in its layout. It
does not. An inactive `Loader` is still a visible, zero-size item, and a
`RowLayout` / `ColumnLayout` puts its `spacing` next to it like any other
child. Measured with PySide6 6.12, offscreen: a `ColumnLayout { spacing:
14 }` holding a live 20px `Loader`, an inactive `Loader` and a 10px item
was 58px tall, which is 20 + 14 + 0 + 14 + 10, not 44.

This came up while making a multi-child branch body a layout (UI21 §2.2,
rule 7, backlog 6d14). TaskApp's view switch is a body whose only child is
an `If` / `Else` pair. Copying the parent's 14px spacing onto that body
would have added a 14px gap below the live view, next to nothing.

**Fix.** A branch body whose only child is an `If` / `Else` pair gets
`spacing: 0`, whatever its host. The same gap still appears wherever an
`If` / `Else` pair sits directly in a spaced layout; that is pre-existing
and not fixed here.

**Do differently.** When a lowering emits a zero-size placeholder into a
Qt layout (an inactive `Loader`, an empty `Item`), assume it costs a
spacing slot unless it is `visible: false`, which layouts skip. Measure it
in a throwaway PySide6 view rather than reasoning from CSS, where an absent
branch takes no gap at all.
