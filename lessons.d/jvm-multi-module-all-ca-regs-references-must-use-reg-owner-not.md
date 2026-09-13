---
category: Python
---

# JVM multi-module: all `__ca_regs` references must use `_reg_owner`, not `self.config.class_name`

Any `field_ref(self.config.class_name, "__ca_regs", ...)` inside helper methods (`__ca_syscall`, etc.) that was added before `external_runtime_class` was introduced must be updated to `_reg_field_ref(...)`. Two `getstatic` calls in `_build_syscall_method` (SYSCALL 1 write-byte, SYSCALL 10 exit) were missed and caused `NoSuchFieldError` at runtime in multi-module mode.
