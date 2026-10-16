---
category: Rust
---

# A reborrowed compiler tuple can rely on auto-deref

CLOC33 focused474e passed its nine private, three CLI and nine ancestry tests,
but strict Clippy rejected ledger.observer(&mut **log, root) at the borrowed
CV tuple seam. Rust already performs that dereference/reborrow from the expected
parameter type. Use ledger.observer(log, root), preserve the same lifetime and
run the unchanged strict lint; do not suppress explicit_auto_deref or call the
focused run green before its final lint command is consumed.
