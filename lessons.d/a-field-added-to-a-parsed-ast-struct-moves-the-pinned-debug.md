---
category: Workspace & package metadata
---

# A field added to a parsed AST struct moves the pinned Debug hash of every v0 specification unless Debug leaves it out

`Parameter` gained `inout: bool` for WX-G. `tests/v0_compat.rs` pins the SHA-256 of
`format!("{:?}", file.declarations)` for the two v0 specifications, so the derived `Debug` of
the struct changed and the "parsed declarations" hash moved, though no v0 document means
anything different. Adding the field to a struct inside `CheckedWarrantModule` would have moved
the checker hash the same way (VER04k 1.3 item 3).

Implement `Debug` by hand and print the new field only when it is set (`debug_struct`, then
`.field("inout", ..)` under `if self.inout`): the text for every existing document is
byte-identical, and the field appears exactly where a v1 document uses it. Keep new checker
data private to the checker instead of adding it to the public result.
