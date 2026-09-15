---
category: QR / format-marker / file-format specifics
---

# Intel 4004 R1 corruption

When `_emit_add_imm`'s source virtual register maps to physical R1, don't clobber R1 as scratch — use R14. Special-case `k=0` as a pure copy: `LD Rsrc; XCH Rdst`.
