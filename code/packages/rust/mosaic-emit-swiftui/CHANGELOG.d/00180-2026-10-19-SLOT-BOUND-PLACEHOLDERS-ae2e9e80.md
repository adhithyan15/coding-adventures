## 2026-10-19 (Slot-bound placeholders)

A `placeholder: slot:` now reaches `TextField` (HostInput, the single-line
legacy `Input`), the `TextEditor` overlay and accessibility fallback (multiline
fields) and the number field (`HostNumberInput`). All three read only literals,
so a slot-bound hint was an empty one here: the note-type editor's card
templates had none on SwiftUI. They share `swift_placeholder_expr`, which
yields the escaped literal or the slot's `String` property. Generated
output for a probe and for the note-type editor passes `swiftc -typecheck`
for macOS 13 and iOS 16.
