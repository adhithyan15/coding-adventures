---
category: TypeScript / JavaScript
---

# `new RegExp(userControlledPattern)` is a ReDoS vector

Cap pattern length (`MAX_PATTERN_LENGTH = 1024` is generous for any real schema) before construction. Document the cap; treat oversize as a validation failure, not an exception. Found in `forme-pipeline-config`'s JSON Schema `pattern` keyword.
