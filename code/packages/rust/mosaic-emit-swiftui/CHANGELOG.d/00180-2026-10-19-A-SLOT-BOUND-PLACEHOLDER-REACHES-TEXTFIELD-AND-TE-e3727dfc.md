## 2026-10-19 (A slot-bound placeholder reaches TextField and TextEditor)

`placeholder : slot: x` on `HostInput` and the legacy `Input` was dropped:
the placeholder was read with a string-literal-only lookup, so every toolkit
`Input` and the note-type editor's fields came out as `TextField("", …)` or
as a `TextEditor` with no overlay, and the degradation report said nothing.
`swift_placeholder` now reads every authored form. A literal stays a
`LocalizedStringKey` literal, as before. A slot, a `For` binding or an
expression becomes `String(describing: …)`, so it is a `String` whatever the
value's Swift type is. That is the `TextField` title; for a `TextEditor` it
is the `Text(verbatim:)` overlay and the fallback `accessibilityLabel`. A slot
or binding name that is not a Swift identifier is an `UnsafeSlotName` error.
