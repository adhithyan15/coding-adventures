---
category: Testing & coverage
---

# Every practised atom in a direct owner needs a body block assessment even when a later lesson owns the frame

Punjabi chapter 197's direct vocabulary owner declared the familiar
verb-last atom in both `requires.knowledge` and `practises.knowledge`, because
the chapter would later place the noun inside an identity frame. Its warm-up
retrieved the frame's two familiar words but assessed only their lexical atoms.
The focused language test passed, while integration and CLI validation rejected
the owner because the practised grammar atom appeared in no body block's
assessment list.

When a lesson declares a familiar structural atom as practised, its own body
must assess that atom even if a later lesson performs the full composition.
Include the atom in the warm-up's `hl-knowledge` assessment when the warm-up
retrieves that structure, or remove it from the lesson's practised set when the
lesson genuinely does not exercise it. Run integration and CLI validation with
the language wrapper before treating a direct-owner test as sufficient.
