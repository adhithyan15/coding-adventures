---
category: Native extensions & FFI
---

# Ruby `QNIL = 0x04` on 64-bit Ruby (USE_FLONUM), not `0x08`

The pre-FLONUM `0x08` causes Ruby to dereference it as an object pointer (klass at `+8` → SIGSEGV at `0x10`). Constants: `QFALSE=0x00, QNIL=0x04, QTRUE=0x14, QUNDEF=0x24`. Confirm against `ruby/internal/special_consts.h`. When a Ruby native ext SIGSEGVs at low addresses like `0x10`, suspect a special-constant bit-pattern bug.
