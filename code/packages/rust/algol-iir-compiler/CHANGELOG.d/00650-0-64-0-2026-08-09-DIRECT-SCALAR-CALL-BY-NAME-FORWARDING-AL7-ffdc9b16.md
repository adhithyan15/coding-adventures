## 0.64.0 — 2026-08-09 — direct scalar call-by-name forwarding (AL7)

Direct scalar name formals can now be forwarded to another direct name call.
The compiler substitutes the stored caller actual and captures each dependency
under a generated global alias before lowering the next specialised sibling, so
the forwarded expression retains its lexical binding even when the callee
reuses the same parameter name. Coverage includes write-through forwarding,
expression forwarding, and nested parameter shadowing.

