---
category: Python
---

# JVM multi-module: exported functions in dep modules need `extra_callable_labels`

`_discover_callable_regions` builds callable names from `CALL` instructions; exported functions that are only called cross-module have no local callers, so they are silently omitted from the class file → `NoSuchMethodError` at runtime. Pass `module.program.module.exports` as `extra_callable_labels` in the JvmBackendConfig.
